import { Selection, type Transaction } from '@tiptap/pm/state'
import { tr } from './i18n'
import { editingProvider, historyFor } from './editing-history'
import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import { CONFIRMED_COMMAND, type Provider, type Transport } from './provider'
import { draftGuard } from './snapshot-operation'

export interface StructuredCommand {
  id: string
  params: Record<string, unknown>
}
/** Prepare in Rust; refuse concurrent input instead of applying a delta to another revision.
 * The owner rechecks the guard before committing; the confirmed update joins participant undo.
 */
export async function executeStructured(
  editor: Editor,
  provider: Provider,
  transport: Transport,
  commands: StructuredCommand[],
  stillCurrent: () => boolean = () => true,
  installed?: () => void,
) {
  const original = Y.encodeStateAsUpdate(provider.doc)
  const selection = editor.state.selection
  let committing = false
  const unchanged = () => {
    const current = Y.encodeStateAsUpdate(provider.doc)
    return (
      !editor.isDestroyed &&
      !editor.view.composing &&
      (committing || stillCurrent()) &&
      selection.eq(editor.state.selection) &&
      original.length === current.length &&
      original.every((value, index) => current[index] === value)
    )
  }
  const check = await draftGuard(provider, unchanged)
  const request = {
    generation: provider.generation,
    revision: provider.revision,
    update: [...original],
    commands,
  }
  const identity = provider.doc.getMap('settings').get('identity')
  const invoke = (command: string) =>
    transport.invoke<{ update: number[]; changed: boolean; guard: { revision: number } }>(command, {
      request,
    })
  const result = await invoke('prepare_structured_commands').catch((error: unknown) => {
    let code: string | undefined
    try {
      code = JSON.parse(String(error)).code
    } catch {
      /* Transport failures carry no documentary code. */
    }
    throw new Error(
      tr(
        code === 'wrong_document' || code === 'stale_revision'
          ? 'Le document a changé. Réessayez.'
          : 'Impossible d’appliquer cette modification.',
      ),
      { cause: error },
    )
  })
  check()
  if (!result.changed) return false
  // Keep the validated checkpoint stable only during the final owner commit.
  // The pending band retains focus; typing remains immediate during preparation.
  const commit = async () => {
    check()
    const publication = await invoke('commit_structured_commands').catch((error: unknown) => {
      throw new Error(tr('Le document a changé. Réessayez.'), { cause: error })
    })
    if (
      provider.generation !== request.generation ||
      provider.doc.getMap('settings').get('identity') !== identity
    )
      return false
    const undo = provider.nativeEditing
      ? undefined
      : yUndoPluginKey.getState(editor.state)?.undoManager
    historyFor(editor)?.stopCapturing()
    undo?.addTrackedOrigin(CONFIRMED_COMMAND)
    undo?.stopCapturing()
    Y.applyUpdate(provider.doc, new Uint8Array(publication.update), CONFIRMED_COMMAND)
    undo?.stopCapturing()
    // Position-preserving operations can restore their local selection before
    // later remote transactions map it. Never guess old positions over a newer revision.
    if (provider.revision <= publication.guard.revision) installed?.()
    provider.revision = Math.max(provider.revision, publication.guard.revision)
    if (provider.nativeEditing) {
      // No raw relay: the owner already published this operation. Fetch counters without clearing.
      await provider.refreshHistory()
    }
    return publication.changed
  }
  committing = true // Inert zones may yield focus to body during the final lock.
  if (provider.nativeEditing) return provider.commitCommand(commit)
  const editable = editor.isEditable
  editor.setEditable(false, false)
  try {
    return await commit()
  } finally {
    if (!editor.isDestroyed) editor.setEditable(editable, false)
  }
}

/** A planned local projection and its typed batch share publication, caret and focus rules.
 * Detached drafts keep local undo; bound zones only install the owner's confirmed delta.
 */
export async function applyPlannedEdit(
  editor: Editor,
  transaction: Transaction,
  commands: StructuredCommand[],
  options: { focus?: boolean; current?: () => boolean; installed?: () => void } = {},
): Promise<boolean> {
  if (!editor.isEditable || editor.view.composing || editor.isDestroyed) return false
  const provider = editingProvider(editor)
  if (!provider) {
    historyFor(editor)?.stopCapturing()
    editor.view.dispatch(transaction.scrollIntoView())
    historyFor(editor)?.stopCapturing()
    options.installed?.()
    if (options.focus !== false) editor.view.focus()
    return transaction.docChanged
  }
  if (!commands.length) return false
  const focused = document.activeElement
  const changed = await executeStructured(
    editor,
    provider,
    provider.transport,
    commands,
    () =>
      (!options.current || options.current()) &&
      [focused, document.body, editor.view.dom].includes(document.activeElement),
    () => {
      editor.view.dispatch(
        editor.state.tr
          .setSelection(Selection.fromJSON(editor.state.doc, transaction.selection.toJSON()))
          .scrollIntoView(),
      )
      options.installed?.()
    },
  )
  if (
    options.focus !== false &&
    !editor.isDestroyed &&
    [focused, document.body, editor.view.dom].includes(document.activeElement)
  )
    editor.view.focus()
  return changed
}

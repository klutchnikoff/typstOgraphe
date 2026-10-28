import { tr } from './i18n'
import { historyFor } from './editing-history'
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
) {
  const original = Y.encodeStateAsUpdate(provider.doc)
  const selection = editor.state.selection
  const unchanged = () => {
    const current = Y.encodeStateAsUpdate(provider.doc)
    return (
      !editor.isDestroyed &&
      !editor.view.composing &&
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
  const editable = editor.isEditable
  editor.setEditable(false, false)
  try {
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
    provider.revision = Math.max(provider.revision, publication.guard.revision)
    if (provider.nativeEditing) {
      // No raw relay: the owner already published this operation. Fetch counters without clearing.
      await provider.refreshHistory()
    }
    return publication.changed
  } finally {
    if (!editor.isDestroyed) editor.setEditable(editable, false)
  }
}

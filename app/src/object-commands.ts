import type { Editor, JSONContent } from '@tiptap/core'
import { Selection, TextSelection, type Transaction } from '@tiptap/pm/state'
import { documentContent } from './shared-inline-marks'
import { normalizeWhitespace } from './document-whitespace'
import { editingProvider, historyFor } from './editing-history'
import { editorRoot, nodePath, type TextTarget } from './editor-targets'
import { executeStructured, type StructuredCommand } from './structured-commands'

export function treeTarget(editor: Editor, pos: number) {
  const root = editorRoot(editor)
  if (!root) throw new Error('Missing structured editor binding.')
  return { root, path: nodePath(editor, pos) }
}
export function inlineTarget(editor: Editor, from: number, to: number): TextTarget {
  const a = editor.state.doc.resolve(from),
    b = editor.state.doc.resolve(to)
  if (!a.parent.inlineContent || !a.sameParent(b)) throw new Error('Invalid inline range.')
  const root = editorRoot(editor)
  if (!root) throw new Error('Missing structured editor binding.')
  return {
    zone: { ...root, path: nodePath(editor, a.before()) },
    from: a.parentOffset,
    to: b.parentOffset,
  }
}
/** Translate one explicit widget transaction into the smallest root splice.
 * The original insertion/conversion policy remains shared with immediate typing;
 * unchanged blocks and all their CRDT identities are outside the command.
 */
export function widgetBatch(editor: Editor, transaction: Transaction): StructuredCommand[] {
  normalizeWhitespace(transaction)
  const before = editor.state.doc.content.content,
    after = transaction.doc.content.content
  let start = 0,
    end = 0
  while (start < before.length && start < after.length && before[start].eq(after[start])) start++
  while (
    end < before.length - start &&
    end < after.length - start &&
    before[before.length - end - 1].eq(after[after.length - end - 1])
  )
    end++
  if (start === before.length && start === after.length) return []
  const root = editorRoot(editor)
  if (!root) throw new Error('Missing structured editor binding.')
  return [
    {
      id: 'block.splice',
      params: {
        parent: { root, path: [] },
        index: start,
        delete_count: before.length - start - end,
        nodes: after.slice(start, after.length - end).map((node) => documentContent(node.toJSON())),
      },
    },
  ]
}
export async function applyObjectEdit(
  editor: Editor,
  transaction: Transaction,
  commands?: StructuredCommand[],
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
  const batch = commands || widgetBatch(editor, transaction)
  if (!batch.length) return false
  const focused = document.activeElement
  const changed = await executeStructured(
    editor,
    provider,
    provider.transport,
    batch,
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
export function runObjectEdit(
  editor: Editor,
  transaction: Transaction,
  commands?: StructuredCommand[],
) {
  void applyObjectEdit(editor, transaction, commands).catch((error: unknown) =>
    editingProvider(editor)?.onError(error),
  )
  return true
}
export function removeObject(editor: Editor, pos: number, focus = true, current?: () => boolean) {
  const node = editor.state.doc.nodeAt(pos)
  if (!node) return Promise.resolve(false)
  const transaction = normalizeWhitespace(editor.state.tr.deleteRange(pos, pos + node.nodeSize))
  const commands = editingProvider(editor)
    ? [
        node.isInline
          ? {
              id: 'text.delete',
              params: { target: inlineTarget(editor, pos, pos + node.nodeSize) },
            }
          : { id: 'block.remove', params: { target: treeTarget(editor, pos) } },
      ]
    : undefined
  return applyObjectEdit(editor, transaction, commands, { focus, current })
}
export async function insertInlineObject(
  editor: Editor,
  content: JSONContent,
  open?: (pos: number, kind: string) => void,
) {
  const node = editor.schema.nodeFromJSON(content)
  if (!node.isInline) return false
  const { from, to } = editor.state.selection
  const transaction = editor.state.tr.replaceWith(from, to, node)
  transaction.setSelection(TextSelection.create(transaction.doc, from + node.nodeSize))
  const commands = editingProvider(editor)
    ? [
        {
          id: 'inline.replace',
          params: { target: inlineTarget(editor, from, to), node: documentContent(node.toJSON()) },
        },
      ]
    : undefined
  const changed = await applyObjectEdit(editor, transaction, commands, { focus: !open })
  if (open && !editor.isDestroyed) {
    const pos = changed ? editor.state.selection.from - node.nodeSize : from
    if (editor.state.doc.nodeAt(pos)?.type === node.type) open(pos, node.type.name)
  }
  return changed
}

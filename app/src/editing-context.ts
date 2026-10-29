import type { Editor } from '@tiptap/core'
import { AllSelection, NodeSelection, Plugin } from '@tiptap/pm/state'
import { Mark } from '@tiptap/pm/model'
import { CellSelection } from '@tiptap/pm/tables'
import { ySyncPluginKey } from '@tiptap/y-tiptap'
import { editorRoot, nodePath, type TextTarget, type TreeRoot } from './editor-targets'
import type { Provider } from './provider'

type TreeTarget = { root: TreeRoot; path: number[] }
export type TextPoint = { zone: TextTarget['zone']; offset: number }
export type ViewSelection =
  | { kind: 'text'; anchor: TextPoint; head: TextPoint }
  | { kind: 'node'; target: TreeTarget }
  | { kind: 'cells'; anchor: TreeTarget; head: TreeTarget }
  | { kind: 'all'; root: TreeRoot }
export type TypingStyle = { type: 'bold' | 'italic' } | { type: 'link'; url: string }
export interface EditingView {
  selection: ViewSelection | null
  typing_styles: TypingStyle[]
}
/** Translate the actual PM selection, preserving direction, atoms and cells.
 * A caret is a UTF-16 point in its textblock, never a global document offset.
 */
export function editingView(editor: Editor): EditingView | undefined {
  const root = editorRoot(editor)
  if (!root || editor.isDestroyed || editor.view.composing) return
  const s = editor.state.selection
  let selection: ViewSelection
  if (s instanceof CellSelection) {
    selection = {
      kind: 'cells',
      anchor: { root, path: nodePath(editor, s.$anchorCell.pos) },
      head: { root, path: nodePath(editor, s.$headCell.pos) },
    }
  } else if (s instanceof NodeSelection) {
    selection = { kind: 'node', target: { root, path: nodePath(editor, s.from) } }
  } else if (s instanceof AllSelection) {
    selection = { kind: 'all', root }
  } else {
    const point = ($pos: typeof s.$anchor): TextPoint => ({
      zone: {
        ...root,
        path: Array.from({ length: $pos.depth }, (_, depth) => $pos.index(depth)),
      },
      offset: $pos.parentOffset,
    })
    if (!s.$anchor.parent.isTextblock || !s.$head.parent.isTextblock)
      return { selection: null, typing_styles: [] }
    selection = { kind: 'text', anchor: point(s.$anchor), head: point(s.$head) }
  }
  // null means inheritance at the caret; [] deliberately stops all styles.
  const marks =
    editor.state.storedMarks ?? (s.empty ? s.$from.marks() : (s.$from.marksAcross(s.$to) ?? []))
  const typing_styles: TypingStyle[] =
    selection.kind === 'text'
      ? marks.flatMap((mark): TypingStyle[] => {
          if (mark.type.name === 'bold' || mark.type.name === 'italic')
            return [{ type: mark.type.name }]
          if (mark.type.name === 'link') return [{ type: 'link', url: mark.attrs.href }]
          return []
        })
      : []
  return { selection, typing_styles }
}
/** PM normally clears storedMarks on a document transaction. A remote change
 * is not a local choice to stop bold/italic/link (including explicit no marks).
 */
export function preserveTypingStyles() {
  return new Plugin({
    appendTransaction(transactions, previous, current) {
      if (
        previous.storedMarks === null ||
        !current.selection.empty ||
        !transactions[0]?.getMeta(ySyncPluginKey)?.isChangeOrigin ||
        (current.storedMarks !== null && Mark.sameSet(previous.storedMarks, current.storedMarks))
      )
        return null
      return current.tr
        .setStoredMarks(previous.storedMarks)
        .setMeta(ySyncPluginKey, { isChangeOrigin: true })
    },
  })
}
/** Only native-bound zones participate. Detached modal drafts keep local state.
 * Remote mapping never advertises a guessed replacement for a deleted zone.
 * Nothing returned by Rust is used to move the local focus or caret.
 */
export function bindEditingContext(editor: Editor, provider: Provider) {
  editor.registerPlugin(preserveTypingStyles())
  const read = () => editingView(editor)
  const focus = () => provider.updateEditingContext(editor, read)
  editor.on('focus', focus)
  const transaction = ({
    transaction,
  }: {
    transaction: import('@tiptap/pm/state').Transaction
  }) => {
    if (transaction.getMeta(ySyncPluginKey)?.isChangeOrigin) return
    if (
      (editor.isFocused || provider.isEditingContextSource(editor)) &&
      (transaction.docChanged || transaction.selectionSet || transaction.storedMarksSet)
    )
      focus()
  }
  editor.on('transaction', transaction)
  editor.on('destroy', () => {
    editor.off('focus', focus)
    editor.off('transaction', transaction)
    provider.releaseEditingContext(editor)
  })
}

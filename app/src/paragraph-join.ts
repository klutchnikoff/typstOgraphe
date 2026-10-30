import type { Editor } from '@tiptap/core'
import { TextSelection } from '@tiptap/pm/state'
import { editingProvider } from './editing-history'
import { editingView } from './editing-context'

const prose = (kind: string) => kind === 'paragraph' || kind === 'heading'
const containers = new Set(['doc', 'quote', 'listItem', 'tableCell', 'tableHeader'])

/** Handle only a collapsed caret at a textblock boundary. A protected boundary
 * is consumed too: ProseMirror must not lift or unwrap a neighboring widget.
 * Intra-block deletion and noncollapsed selections keep their usual handlers.
 */
export function joinParagraphs(editor: Editor, backwards: boolean, trailingParagraph: boolean) {
  const { selection, storedMarks } = editor.state
  if (editor.view.composing || !(selection instanceof TextSelection) || !selection.empty)
    return false
  const { $from } = selection
  if (
    !$from.parent.isTextblock ||
    $from.parentOffset !== (backwards ? 0 : $from.parent.content.size)
  )
    return false
  if (!prose($from.parent.type.name) || !containers.has($from.node($from.depth - 1).type.name))
    return true
  const parent = $from.node($from.depth - 1)
  const index = $from.index($from.depth - 1) - (backwards ? 1 : 0)
  if (index < 0 || index + 1 >= parent.childCount) return true
  const left = parent.child(index),
    right = parent.child(index + 1)
  if (!prose(left.type.name) || !prose(right.type.name)) return true
  if (
    trailingParagraph &&
    $from.depth === 1 &&
    index + 2 === parent.childCount &&
    right.type.name === 'paragraph' &&
    !right.content.size
  )
    return true
  const boundary = backwards ? $from.before() : $from.after()
  const leftStart = boundary - left.nodeSize
  const tr = editor.state.tr
  if (!left.content.size) tr.delete(leftStart, boundary)
  else tr.join(boundary)
  const seam = leftStart + 1 + left.content.size
  tr.setSelection(TextSelection.create(tr.doc, seam))
  // null inherits the left context; [] explicitly stops every typing style.
  tr.setStoredMarks(storedMarks ?? tr.selection.$from.marks())
  const provider = editingProvider(editor),
    before = editingView(editor)
  const dispatch = () => editor.view.dispatch(tr.scrollIntoView())
  if (provider && before) provider.withHistoryView(before, dispatch)
  else dispatch()
  return true
}

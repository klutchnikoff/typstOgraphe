import type { Editor } from '@tiptap/core'
import { AllSelection, NodeSelection, TextSelection } from '@tiptap/pm/state'
import { CellSelection } from '@tiptap/pm/tables'
import { editorRoot, positionAtPath, type TreeRoot } from './editor-targets'
import type { HistoryState } from './provider'

/** Restore only an exact live zone in the initiating view. Never choose a
 * neighboring field, reopen a modal, focus another window or guess a position.
 */
export function restoreHistorySelection(editor: Editor, snapshot: HistoryState) {
  const selection = snapshot.selection
  const root = editorRoot(editor)
  if (!root || !selection || snapshot.selection_lost) return false
  const sameRoot = (other: TreeRoot) =>
    root.kind === other.kind &&
    (root.kind === 'body' ||
      (other.kind === 'record' &&
        root.collection === other.collection &&
        root.id === other.id &&
        root.field === other.field))
  const position = (path: number[]) => {
    if (!path.length) return
    try {
      const pos = positionAtPath(editor, path),
        node = editor.state.doc.nodeAt(pos)
      return node ? { pos, node } : undefined
    } catch {
      return undefined
    }
  }
  const point = (point: import('./editing-context').TextPoint) => {
    if (!sameRoot(point.zone)) return
    const found = position(point.zone.path)
    if (!found?.node.isTextblock || point.offset < 0 || point.offset > found.node.content.size)
      return
    return found.pos + 1 + point.offset
  }
  let next
  if (selection.kind === 'text') {
    const anchor = point(selection.anchor),
      head = point(selection.head)
    if (anchor === undefined || head === undefined) return false
    next = TextSelection.create(editor.state.doc, anchor, head)
  } else if (selection.kind === 'all') {
    if (!sameRoot(selection.root)) return false
    next = new AllSelection(editor.state.doc)
  } else if (selection.kind === 'node') {
    if (!sameRoot(selection.target.root)) return false
    const found = position(selection.target.path)
    if (!found || !NodeSelection.isSelectable(found.node)) return false
    next = NodeSelection.create(editor.state.doc, found.pos)
  } else {
    if (!sameRoot(selection.anchor.root) || !sameRoot(selection.head.root)) return false
    const a = position(selection.anchor.path),
      h = position(selection.head.path)
    if (
      !a ||
      !h ||
      ![a, h].every((p) => ['cell', 'header_cell'].includes(p.node.type.spec.tableRole || ''))
    )
      return false
    next = CellSelection.create(editor.state.doc, a.pos, h.pos)
  }
  const marks = (snapshot.typing_styles || []).flatMap((style) => {
    const type = editor.schema.marks[style.type]
    return type ? [type.create(style.type === 'link' ? { href: style.url } : undefined)] : []
  })
  editor.view.dispatch(editor.state.tr.setSelection(next).setStoredMarks(marks).scrollIntoView())
  return true
}

import type { Editor } from '@tiptap/core'
import { CellSelection } from '@tiptap/pm/tables'
/** Cell selections point before a cell; keyboard actions need its interior path. */
export function tableCursor(editor: Editor) {
  const selection = editor.state.selection
  if (selection instanceof CellSelection)
    return editor.state.doc.resolve(selection.$headCell.pos + 1)
  const pos = selection.$from
  return ['tableCell', 'tableHeader'].includes(pos.nodeAfter?.type.name || '')
    ? editor.state.doc.resolve(pos.pos + 1)
    : pos
}

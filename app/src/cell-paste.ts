import type { Editor } from '@tiptap/core'
import { Fragment, Slice, type Node as PMNode } from '@tiptap/pm/model'
import { CellSelection, selectedRect } from '@tiptap/pm/tables'
import { editorRoot, nodePath } from './editor-targets'
import { clipboardParagraphs } from './clipboard-content'
import { documentContent } from './shared-inline-marks'
import { normalizeWhitespace } from './document-whitespace'
import type { StructuredCommand } from './structured-commands'

/** Same clipping/repetition policy as rectangular ProseMirror paste, but replace
 * contents only: header roles, cell identities, dimensions and caption survive. */
export function cellPastePlan(editor: Editor, slice: Slice) {
  const selection = editor.state.selection
  if (!(selection instanceof CellSelection)) return
  const grid: PMNode[][] = []
  const visit = (node: PMNode) => {
    if (node.type.spec.tableRole === 'row') {
      grid.push([...node.content.content])
      return
    }
    if (!node.isInline) node.forEach(visit)
  }
  slice.content.forEach(visit)
  const rect = selectedRect(editor.state)
  const cells: { node: PMNode; pos: number; content: PMNode[] }[] = []
  selection.forEachCell((node, pos) => {
    const location = rect.map.findCell(pos - rect.tableStart)
    const row = grid.length ? grid[(location.top - rect.top) % grid.length] : undefined
    const copied = row?.length ? row[(location.left - rect.left) % row.length] : undefined
    const content = clipboardParagraphs(
      copied ? new Slice(Fragment.from(copied), 0, 0) : slice,
      editor,
    )
    cells.push({ node, pos, content })
  })
  const root = editorRoot(editor),
    tr = editor.state.tr,
    commands: StructuredCommand[] = []
  for (const cell of cells.sort((a, b) => b.pos - a.pos)) {
    tr.replaceWith(cell.pos + 1, cell.pos + cell.node.nodeSize - 1, cell.content)
    if (root)
      commands.push({
        id: 'block.splice',
        params: {
          parent: { root, path: nodePath(editor, cell.pos) },
          index: 0,
          delete_count: cell.node.childCount,
          nodes: cell.content.map((node) => documentContent(node.toJSON())),
        },
      })
  }
  tr.setSelection(selection.getBookmark().map(tr.mapping).resolve(tr.doc))
  normalizeWhitespace(tr)
  return { transaction: tr, commands }
}

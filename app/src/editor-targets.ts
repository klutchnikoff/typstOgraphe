import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'

export type RecordRoot = { kind: 'record'; collection: string; id: string; field: string }
export type TreeRoot = { kind: 'body' } | RecordRoot
export interface TextTarget {
  zone: TreeRoot & { path: number[] }
  from: number
  to: number
}
const roots = new WeakMap<Editor, TreeRoot>()
export function bindEditorTarget(editor: Editor, root: TreeRoot) {
  roots.set(editor, root)
}
export function editorRoot(editor: Editor) {
  return roots.get(editor)
}
/** Public projected paths, never XML child indices or offsets in a flattened document. */
export function nodePath(editor: Editor, pos: number) {
  const resolved = editor.state.doc.resolve(pos)
  return Array.from({ length: resolved.depth + 1 }, (_, depth) => resolved.index(depth))
}
export function selectedTextblocks(
  editor: Editor,
  range: { from: number; to: number } = editor.state.selection,
): { node: PMNode; pos: number; path: number[] }[] {
  const blocks: { node: PMNode; pos: number; path: number[] }[] = []
  editor.state.doc.nodesBetween(range.from, range.to, (node, pos) => {
    if (!node.isTextblock) return
    blocks.push({ node, pos, path: nodePath(editor, pos) })
    return false
  })
  return blocks
}
/** Split a visual selection into an atomic batch of independent UTF-16 zones.
 * Inline objects count for one; their displayed labels never affect addressing.
 */
export function textTargets(
  editor: Editor,
  range: { from: number; to: number } = editor.state.selection,
): TextTarget[] {
  const root = editorRoot(editor)
  if (!root) throw new Error('Missing structured editor binding.')
  return selectedTextblocks(editor, range).flatMap(({ node, pos, path }) => {
    const from = Math.max(0, range.from - pos - 1)
    const to = Math.min(node.content.size, range.to - pos - 1)
    return from < to ? [{ zone: { ...root, path }, from, to }] : []
  })
}

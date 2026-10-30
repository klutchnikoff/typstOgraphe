import type { Editor } from '@tiptap/core'
import type { Node as PMNode, Slice } from '@tiptap/pm/model'

/** Compatible editorial content only. Wrappers never become nested widgets. */
function parts(slice: Slice): PMNode[][] {
  const result: PMNode[][] = []
  const visit = (node: PMNode) => {
    if (node.isTextblock) result.push([...node.content.content])
    else if (node.isInline) {
      if (!result.length) result.push([])
      result[result.length - 1].push(node)
    } else node.forEach(visit)
  }
  slice.content.forEach(visit)
  return result
}
export function clipboardInline(slice: Slice, editor: Editor, singleLine = false) {
  return parts(slice).flatMap((part, index) => [
    ...(index
      ? [singleLine ? editor.schema.text(' ') : editor.schema.nodes.softBreak.create()]
      : []),
    ...part.map((node) =>
      singleLine && ['softBreak', 'hardBreak'].includes(node.type.name)
        ? editor.schema.text(' ')
        : node,
    ),
  ])
}
export function clipboardParagraphs(slice: Slice, editor: Editor) {
  const content = parts(slice)
  return (content.length ? content : [[]]).map((part) =>
    editor.schema.nodes.paragraph.create(null, part),
  )
}

import { Mark, type JSONContent } from '@tiptap/core'
import type { ContentHighlight } from './comparison-types'

/** Static rendering only: these marks never enter an editor, Yjs or a snapshot. */
export const ComparisonChange = Mark.create({
  name: 'comparisonChange',
  addAttributes: () => ({ kind: { default: 'insert', rendered: false } }),
  renderHTML: ({ mark }) => [mark.attrs.kind === 'delete' ? 'del' : 'ins', 0],
})

export function highlightedContent(
  document: JSONContent,
  highlights: ContentHighlight[],
  kind: 'delete' | 'insert',
): JSONContent {
  const ranges = new Map<string, ContentHighlight[]>()
  for (const range of highlights) {
    const key = range.path.join('/')
    ranges.set(key, [...(ranges.get(key) || []), range])
  }
  const mark = { type: 'comparisonChange', attrs: { kind } }
  function visit(node: JSONContent, path: number[]): JSONContent[] {
    const changes = ranges.get(path.join('/'))
    if (changes?.length && node.type === 'text') {
      const text = node.text || '',
        result: JSONContent[] = []
      let cursor = 0
      for (const range of changes) {
        const from = Math.max(cursor, range.from),
          to = Math.min(text.length, range.to)
        if (to <= from) continue
        if (from > cursor) result.push({ ...node, text: text.slice(cursor, from) })
        result.push({ ...node, text: text.slice(from, to), marks: [...(node.marks || []), mark] })
        cursor = to
      }
      if (cursor < text.length) result.push({ ...node, text: text.slice(cursor) })
      return result
    }
    return [
      {
        ...node,
        ...(changes?.length ? { marks: [...(node.marks || []), mark] } : {}),
        ...(node.content
          ? { content: node.content.flatMap((child, i) => visit(child, [...path, i])) }
          : {}),
      },
    ]
  }
  return visit(document, [])[0]
}

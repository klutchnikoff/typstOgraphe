import { Extension, type JSONContent } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { ySyncPluginKey } from '@tiptap/y-tiptap'

// y-tiptap stores text marks, but serializes non-text inline nodes using attributes.
// Keep the public document's `marks` intact through that shared representation.
const attribute = '__marks'
const key = new PluginKey('shared-inline-marks')

/** Public document values carry marks, never binding implementation attributes. */
export function documentContent(value: JSONContent): JSONContent {
  const result = { ...value }
  if (value.attrs && attribute in value.attrs) {
    const { [attribute]: marks, ...attrs } = value.attrs
    if (Object.keys(attrs).length) result.attrs = attrs
    else delete result.attrs
    if (!result.marks?.length && Array.isArray(marks) && marks.length) result.marks = marks
  }
  if (value.content) result.content = value.content.map(documentContent)
  return result
}
export const SharedInlineMarks = Extension.create({
  name: 'sharedInlineMarks',
  addGlobalAttributes() {
    return [
      {
        types: ['footnote', 'citation', 'softBreak', 'hardBreak'],
        attributes: { [attribute]: { default: null, rendered: false } },
      },
    ]
  },
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key,
        appendTransaction(transactions, previous, current) {
          const changed = transactions.filter((tr) => tr.docChanged && !tr.getMeta(key))
          if (!changed.length) return null
          const start = previous.doc.content.findDiffStart(current.doc.content)
          if (start === null) return null
          const end = previous.doc.content.findDiffEnd(current.doc.content)
          const remote = changed.some((tr) => tr.getMeta(ySyncPluginKey)?.isChangeOrigin)
          const tr = current.tr
          current.doc.nodesBetween(
            Math.max(0, start - 1),
            Math.min(current.doc.content.size, Math.max(start, end?.b ?? start) + 1),
            (node, pos) => {
              if (!node.isInline || node.isText || !(attribute in node.attrs)) return
              if (remote) {
                const marks = (node.attrs[attribute] || []).map((value: unknown) =>
                  current.schema.markFromJSON(value),
                )
                if (!node.sameMarkup(node.type.create(node.attrs, undefined, marks)))
                  tr.setNodeMarkup(pos, undefined, node.attrs, marks)
              } else {
                const marks = node.marks.length ? node.marks.map((mark) => mark.toJSON()) : null
                if (JSON.stringify(marks) !== JSON.stringify(node.attrs[attribute]))
                  tr.setNodeMarkup(
                    pos,
                    undefined,
                    { ...node.attrs, [attribute]: marks },
                    node.marks,
                  )
              }
            },
          )
          if (!tr.docChanged) return null
          tr.setMeta(key, true)
          // Restoring marks already carried by an incoming update is a projection,
          // not a second shared edit or another undo step.
          if (remote) tr.setMeta(ySyncPluginKey, { isChangeOrigin: true })
          return tr
        },
      }),
    ]
  },
})

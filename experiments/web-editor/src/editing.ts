import { Extension, type Editor, type JSONContent } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Plugin, TextSelection } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { widgetTarget } from './widgets'

export type ListKind = 'bulletList' | 'orderedList' | 'descriptionList'
export const listKinds: ListKind[] = ['bulletList', 'orderedList', 'descriptionList']
const paragraph = (content: JSONContent[] = []): JSONContent => ({ type: 'paragraph', content })
export function listContent(kind: ListKind, content: JSONContent[] = []): JSONContent {
  return {
    type: kind,
    content: [
      kind === 'descriptionList'
        ? {
            type: 'descriptionItem',
            content: [{ type: 'descriptionTerm' }, { type: 'descriptionDefinition', content }],
          }
        : { type: 'listItem', content: [paragraph(content)] },
    ],
  }
}
/** Insert at the cursor, converting only an explicit selection, never its containing paragraph. */
export function insertBlock(editor: Editor, content: JSONContent, range = editor.state.selection) {
  let { from, to } = range
  const tr = editor.state.tr
  const $from = tr.doc.resolve(from)
  if (from === to && $from.nodeBefore?.type.name === 'softBreak') {
    tr.delete(--from, to)
    to = from
  }
  const node = editor.schema.nodeFromJSON(content)
  tr.replaceRangeWith(from, to, node)
  let inserted = -1
  tr.doc.descendants((candidate, pos) => {
    if (candidate === node) inserted = pos
  })
  if (inserted < 0) return false
  tr.setSelection(TextSelection.near(tr.doc.resolve(inserted + 1)))
  editor.view.dispatch(tr.scrollIntoView())
  editor.commands.focus()
  return true
}
export function convertList(node: PMNode, kind: ListKind): JSONContent {
  const items: JSONContent[][] = []
  node.forEach((item) => {
    const inline: JSONContent[] = []
    item.forEach((part, _, index) => {
      if (index && part.content.size && inline.length)
        inline.push({ type: 'text', text: node.type.name === 'descriptionList' ? ' : ' : ' ' })
      inline.push(...(part.content.toJSON() || []))
    })
    items.push(inline)
  })
  return {
    type: kind,
    attrs: kind === 'orderedList' ? { start: node.attrs.start || 1 } : {},
    content: items.map((content) => listContent(kind, content).content![0]),
  }
}
export function insertList(editor: Editor, kind: ListKind) {
  const target = widgetTarget(editor)
  if (target && listKinds.includes(target.node.type.name as ListKind)) {
    const replacement =
      target.node.type.name === kind
        ? target.node.content.content.flatMap((item, index) =>
            item.type.name === 'descriptionItem'
              ? [
                  paragraph(
                    convertList(target.node, 'bulletList').content![index].content![0].content,
                  ),
                ]
              : item.content.toJSON(),
          )
        : convertList(target.node, kind)
    return editor
      .chain()
      .focus()
      .insertContentAt({ from: target.pos, to: target.pos + target.node.nodeSize }, replacement)
      .run()
  }
  const { from, to, $from, $to } = editor.state.selection
  const content: JSONContent[] = []
  if (from !== to) {
    const selected = editor.state.doc.slice(from, to).content
    if ($from.sameParent($to)) content.push(...(selected.toJSON() || []))
    else
      selected.forEach((node) => {
        if (content.length) content.push({ type: 'softBreak' })
        content.push(...(node.isText ? [node.toJSON()] : node.content.toJSON() || []))
      })
  }
  return insertBlock(editor, listContent(kind, content))
}
export function insertQuote(editor: Editor) {
  const { from, to } = editor.state.selection
  const content: JSONContent[] = []
  if (from !== to)
    editor.state.doc.slice(from, to).content.forEach((node) => {
      if (node.isText || node.isInline) {
        if (!content.length) content.push(paragraph())
        content[content.length - 1].content!.push(node.toJSON())
      } else if (node.isTextblock) content.push(paragraph(node.content.toJSON() || []))
    })
  return insertBlock(editor, {
    type: 'quote',
    content: [...(content.length ? content : [paragraph()]), { type: 'quoteAttribution' }],
  })
}
function backspaceInList(editor: Editor) {
  const { $from, empty } = editor.state.selection
  if (!empty || $from.parentOffset !== 0) return false
  let depth = $from.depth
  while (depth > 0 && !['listItem', 'descriptionItem'].includes($from.node(depth).type.name))
    depth--
  if (!depth) return false
  if ($from.node(depth).type.name === 'descriptionItem' && $from.index(depth) === 1) {
    editor.commands.setTextSelection(
      TextSelection.near(editor.state.doc.resolve($from.before()), -1),
    )
    return true
  }
  if ($from.index(depth) !== 0) return false
  const item = $from.node(depth),
    list = $from.node(depth - 1),
    index = $from.index(depth - 1)
  if (index > 0) {
    const previous = list.child(index - 1),
      end = $from.before(depth)
    if (!item.textContent) {
      const tr = editor.state.tr.delete(end, end + item.nodeSize)
      tr.setSelection(TextSelection.near(tr.doc.resolve(end), -1))
      editor.view.dispatch(tr.scrollIntoView())
    } else if (item.type.name === 'listItem') {
      const parts = previous.content.content.slice()
      const last = parts.pop()!
      const joined = previous.type.create(previous.attrs, [
        ...parts,
        last.copy(last.content.append(item.firstChild!.content)),
        ...item.content.content.slice(1),
      ])
      const start = end - previous.nodeSize,
        cursor = end - 2
      const tr = editor.state.tr.replaceWith(start, end + item.nodeSize, joined)
      tr.setSelection(TextSelection.create(tr.doc, cursor))
      editor.view.dispatch(tr.scrollIntoView())
    } else editor.commands.setTextSelection(end - 2)
    return true
  }
  // At the first item, return to preceding prose without unwrapping the widget.
  const before = $from.before(depth - 1)
  if (before > 0)
    editor.commands.setTextSelection(TextSelection.near(editor.state.doc.resolve(before), -1))
  return true
}
export const DocumentInput = Extension.create({
  name: 'documentInput',
  priority: 1100,
  addKeyboardShortcuts() {
    return {
      Backspace: () => !this.editor.view.composing && backspaceInList(this.editor),
      Enter: () => {
        if (this.editor.view.composing) return false
        const { selection } = this.editor.state,
          { $from } = selection
        if ($from.parent.type.name !== 'paragraph') return false
        const parent = $from.node($from.depth - 1).type.name
        if (!['doc', 'quote'].includes(parent)) return false
        if (!$from.parent.content.size && selection.empty) return true
        if (selection.empty && $from.nodeBefore?.type.name === 'softBreak') {
          const pos = $from.pos - 1
          const tr = this.editor.state.tr
            .delete(pos, $from.pos)
            .split(pos, 1, [{ type: this.editor.schema.nodes.paragraph }])
          tr.setSelection(TextSelection.create(tr.doc, pos + 2))
          tr.ensureMarks(this.editor.state.storedMarks || $from.marks())
          this.editor.view.dispatch(tr.scrollIntoView())
          return true
        }
        return this.editor.commands.insertContent({ type: 'softBreak' })
      },
    }
  },
  addProseMirrorPlugins() {
    const editor = this.editor
    return [
      new Plugin({
        props: {
          handleTextInput(view, from, to, text) {
            if (view.composing || text !== ' ' || from !== to) return false
            const $from = view.state.doc.resolve(from)
            if ($from.parent.type.name !== 'paragraph' || widgetTarget(editor)) return false
            let start = $from.start()
            $from.parent.forEach((node, offset) => {
              if (
                offset < $from.parentOffset &&
                ['softBreak', 'hardBreak'].includes(node.type.name)
              )
                start = $from.start() + offset + node.nodeSize
            })
            const prefix = view.state.doc.textBetween(start, from)
            const match = /^(?:-|([1-9]\d*)\.)$/.exec(prefix)
            if (!match) return false
            const kind = match[1] ? 'orderedList' : 'bulletList'
            const tr = view.state.tr.delete(start, from)
            view.dispatch(tr.setSelection(TextSelection.create(tr.doc, start)))
            const content = listContent(kind)
            if (match[1]) content.attrs = { start: Number(match[1]) }
            return insertBlock(editor, content)
          },
          decorations(state) {
            const markers: Decoration[] = []
            const visit = (parent: PMNode, pos: number) => {
              parent.forEach((node, offset, index) => {
                if (
                  index &&
                  node.type.name === 'paragraph' &&
                  parent.child(index - 1).type.name === 'paragraph'
                )
                  markers.push(
                    Decoration.widget(
                      pos + 1 + offset,
                      () => {
                        const marker = document.createElement('div')
                        marker.className = 'paragraph-marker'
                        marker.textContent = '¶'
                        marker.setAttribute('aria-hidden', 'true')
                        return marker
                      },
                      { side: -1 },
                    ),
                  )
                if (node.childCount) visit(node, pos + 1 + offset)
              })
            }
            visit(state.doc, -1)
            return DecorationSet.create(state.doc, markers)
          },
        },
      }),
    ]
  },
})

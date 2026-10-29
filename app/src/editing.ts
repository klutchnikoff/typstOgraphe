import { historyFor } from './editing-history'
import { Extension, Node, markInputRule, type Editor, type JSONContent } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Plugin, TextSelection } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { widgetTarget, widgetAnnotation, resumeAfterWidget } from './widgets'
import { lineBreak } from './document-whitespace'
import { runObjectEdit } from './object-commands'

/** Shared Typst-style inline input, also used by named editorial zones. */
export const TypstInlineInput = Extension.create({
  name: 'typstInlineInput',
  priority: 1000,
  addInputRules() {
    return [markInputRule({ find: /(?:^|\s)(\*([^*]+)\*)$/, type: this.editor.schema.marks.bold })]
  },
})
export const SoftBreak = Node.create({
  name: 'softBreak',
  group: 'inline',
  inline: true,
  selectable: false,
  parseHTML: () => [{ tag: 'br[data-soft-break]' }],
  renderHTML: () => ['br', { 'data-soft-break': '' }],
})

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
type Range = { from: number; to: number }
export function canInsertBlock(editor: Editor, range: Range = editor.state.selection) {
  if (range.from < 0 || range.to < range.from || range.to > editor.state.doc.content.size)
    return false
  return [range.from, range.to].every((pos) => {
    const $pos = editor.state.doc.resolve(pos)
    return (
      $pos.depth === 0 ||
      ($pos.depth === 1 && ['paragraph', 'heading'].includes($pos.parent.type.name))
    )
  })
}
/** One insertion policy for every widget: split at the cursor, no empty line before, one after. */
export function planBlockInsertion(
  editor: Editor,
  content: JSONContent,
  range: Range = editor.state.selection,
) {
  if (!canInsertBlock(editor, range)) return undefined
  const tr = editor.state.tr
  const $from = tr.doc.resolve(range.from),
    $to = tr.doc.resolve(range.to)
  const node = editor.schema.nodeFromJSON(content)
  const nodes: PMNode[] = []
  const start = $from.depth ? $from.before() : range.from
  const end = $to.depth ? $to.after() : range.to
  if ($from.depth) {
    let left = $from.parent.content.cut(0, $from.parentOffset)
    if (['softBreak', 'hardBreak'].includes(left.lastChild?.type.name || ''))
      left = left.cut(0, left.size - 1)
    if (left.size) nodes.push($from.parent.copy(left))
  }
  const inserted = start + nodes.reduce((sum, part) => sum + part.nodeSize, 0)
  nodes.push(node)
  let right = $to.depth ? $to.parent.content.cut($to.parentOffset) : null
  if (right && ['softBreak', 'hardBreak'].includes(right.firstChild?.type.name || ''))
    right = right.cut(1)
  if (right?.size) nodes.push($to.parent.copy(right))
  historyFor(editor)?.stopCapturing()
  tr.replaceWith(start, end, nodes)
  resumeAfterWidget(tr, inserted + node.nodeSize)
  tr.setSelection(TextSelection.near(tr.doc.resolve(inserted + 1)))
  return tr
}
export function insertBlock(
  editor: Editor,
  content: JSONContent,
  range: Range = editor.state.selection,
  immediate = false,
) {
  const tr = planBlockInsertion(editor, content, range)
  if (!tr) return false
  if (!immediate) return runObjectEdit(editor, tr)
  editor.view.dispatch(tr.scrollIntoView())
  historyFor(editor)?.stopCapturing()
  editor.view.focus()
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
    const nodes = (Array.isArray(replacement) ? replacement : [replacement]).map((node) =>
      editor.schema.nodeFromJSON(node),
    )
    return runObjectEdit(
      editor,
      editor.state.tr.replaceWith(target.pos, target.pos + target.node.nodeSize, nodes),
    )
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
/** Reuse an existing line on either side of a paragraph boundary as well as inline. */
function lineAhead(editor: Editor) {
  const { empty, $from } = editor.state.selection
  if (!empty) return null
  if (lineBreak($from.nodeAfter)) return $from.pos + $from.nodeAfter!.nodeSize
  if (!$from.parent.isTextblock || $from.parentOffset !== $from.parent.content.size) return null
  const parent = $from.node($from.depth - 1)
  if (!['doc', 'quote', 'tableCell', 'tableHeader'].includes(parent.type.name)) return null
  const index = $from.index($from.depth - 1) + 1,
    next = parent.maybeChild(index)
  if (next?.type.name !== 'paragraph') return null
  // The final resume paragraph is a placeholder: a first return must remain a Typst space.
  if (parent.type.name === 'doc' && index === parent.childCount - 1 && !next.content.size)
    return null
  return $from.after() + 1
}
function atDocumentStart(editor: Editor) {
  const { empty, $from } = editor.state.selection
  return (
    empty &&
    $from.pos === 1 &&
    $from.depth === 1 &&
    ['paragraph', 'heading'].includes($from.parent.type.name)
  )
}
export const DocumentInput = Extension.create({
  name: 'documentInput',
  priority: 1100,
  addKeyboardShortcuts() {
    return {
      Backspace: () => !this.editor.view.composing && backspaceInList(this.editor),
      'Shift-Enter': () => {
        if (this.editor.view.composing) return false
        if (atDocumentStart(this.editor)) return true
        const { empty, $from } = this.editor.state.selection
        const ahead = lineAhead(this.editor)
        if (ahead !== null) {
          this.editor.commands.setTextSelection(ahead)
          return true
        }
        if (empty && (lineBreak($from.nodeBefore) || !$from.parent.content.size)) return true
        return this.editor.commands.setHardBreak()
      },
      Enter: () => {
        if (this.editor.view.composing) return false
        if (atDocumentStart(this.editor)) return true
        const { selection } = this.editor.state,
          { $from } = selection
        const widget = widgetTarget(this.editor)
        if (widget && widgetAnnotation(widget)?.kind === $from.parent.type.name) {
          if (selection.empty && (!$from.parent.content.size || lineBreak($from.nodeBefore))) {
            const tr = this.editor.state.tr
            if (lineBreak($from.nodeBefore)) tr.delete($from.pos - 1, $from.pos)
            this.editor.view.dispatch(
              resumeAfterWidget(
                tr,
                tr.mapping.map(widget.pos + widget.node.nodeSize),
              ).scrollIntoView(),
            )
            return true
          }
          if (selection.empty && lineBreak($from.nodeAfter))
            return this.editor.commands.setTextSelection($from.pos + 1)
          return this.editor.commands.insertContent({ type: 'softBreak' })
        }
        if ($from.parent.type.name !== 'paragraph') return false
        if (
          selection.empty &&
          !$from.parent.content.size &&
          $from.depth > 1 &&
          $from.node($from.depth - 1).type.name === 'listItem' &&
          !$from.node($from.depth - 1).textContent
        ) {
          const depth = $from.depth - 1,
            list = $from.node(depth - 1)
          const listEnd = $from.after(depth - 1)
          const only = list.childCount === 1
          const tr = this.editor.state.tr.delete(
            only ? $from.before(depth - 1) : $from.before(depth),
            only ? listEnd : $from.after(depth),
          )
          this.editor.view.dispatch(resumeAfterWidget(tr, tr.mapping.map(listEnd)).scrollIntoView())
          return true
        }
        const parent = $from.node($from.depth - 1).type.name
        if (!['doc', 'quote', 'tableCell', 'tableHeader'].includes(parent)) return false
        if (!$from.parent.content.size && selection.empty) return true
        const ahead = lineAhead(this.editor)
        if (ahead !== null) {
          this.editor.commands.setTextSelection(ahead)
          return true
        }
        if (selection.empty && lineBreak($from.nodeBefore)) {
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
            if (!editor.schema.nodes[kind]) return false
            const content = listContent(kind)
            if (match[1]) content.attrs = { start: Number(match[1]) }
            return insertBlock(editor, content, { from: start, to: from }, true)
          },
          decorations(state) {
            const markers: Decoration[] = []
            const visit = (parent: PMNode, pos: number) => {
              parent.forEach((node, offset, index) => {
                if (
                  index &&
                  node.type.name === 'paragraph' &&
                  parent.child(index - 1).type.name === 'paragraph' &&
                  parent.child(index - 1).content.size > 0 &&
                  !(
                    parent.type.name === 'doc' &&
                    index === parent.childCount - 1 &&
                    node.content.size === 0 &&
                    state.selection.from !== pos + 2 + offset
                  )
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

import { tableCursor } from './table-selection'
import { tr } from './i18n'
import { Node, type Editor, type JSONContent } from '@tiptap/core'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import { BulletList, OrderedList } from '@tiptap/extension-list'
import {
  isWidgetKind,
  widgetFrame,
  type WidgetKind,
  type WidgetAction,
  type WidgetPresentations,
} from './widget-frame'
import type { Node as PMNode } from '@tiptap/pm/model'
import { TextSelection, type Transaction } from '@tiptap/pm/state'
export function widgetView(
  kind: WidgetKind,
  tag: string,
  properties: WidgetAction,
  presentations: WidgetPresentations,
) {
  return ({ getPos, node }: { getPos: () => number | undefined; node: PMNode }) => {
    const frame = widgetFrame(kind, getPos, properties, undefined, presentations[kind])
    const { dom } = frame
    const contentDOM = document.createElement(tag)
    const update = (next: PMNode) => {
      if (next.type.name !== kind) return false
      if (kind === 'orderedList') contentDOM.setAttribute('start', String(next.attrs.start))
      return true
    }
    update(node)
    dom.append(contentDOM)
    return {
      dom,
      ...frame.content(contentDOM),
      update,
    }
  }
}
export const simpleWidgets = (properties: WidgetAction, presentations: WidgetPresentations) => [
  ...([BulletList, OrderedList] as const).map((extension) =>
    extension.extend({
      addInputRules: () => [],
      addKeyboardShortcuts: () => ({}), // Les raccourcis publics viennent du registre.
      addAttributes() {
        const attributes = this.parent?.() || {}
        // Le style de numérotation appartient au modèle, pas au contenu éditable.
        delete attributes.type
        return attributes
      },
      addNodeView: () =>
        widgetView(
          extension.name === 'orderedList' ? 'orderedList' : 'bulletList',
          extension.name === 'orderedList' ? 'ol' : 'ul',
          properties,
          presentations,
        ),
    }),
  ),
  Node.create({
    name: 'quote',
    addNodeView: () => widgetView('quote', 'blockquote', properties, presentations),
    group: 'block',
    content: 'paragraph+ quoteAttribution',
    isolating: true,
    defining: true,
    parseHTML: () => [{ tag: 'blockquote' }],
    renderHTML: () => ['blockquote', 0],
  }),
  Node.create({
    name: 'quoteAttribution',
    content: 'inline*',
    parseHTML: () => [{ tag: 'cite[data-attribution]' }],
    renderHTML: () => [
      'cite',
      { 'data-attribution': '', 'data-placeholder': tr('Attribution / source (facultatif)') },
      0,
    ],
  }),
  Node.create({
    name: 'descriptionList',
    addNodeView: () => widgetView('descriptionList', 'dl', properties, presentations),
    group: 'block',
    content: 'descriptionItem+',
    isolating: true,
    defining: true,
    parseHTML: () => [{ tag: 'dl' }],
    renderHTML: () => ['dl', 0],
  }),
  Node.create({
    name: 'descriptionItem',
    content: 'descriptionTerm descriptionDefinition',
    defining: true,
    parseHTML: () => [{ tag: 'div[data-description-item]' }],
    renderHTML: () => ['div', { 'data-description-item': '' }, 0],
  }),
  ...(['descriptionTerm', 'descriptionDefinition'] as const).map((name, index) =>
    Node.create({
      name,
      content: 'inline*',
      parseHTML: () => [{ tag: index ? 'dd' : 'dt' }],
      renderHTML: () => [
        index ? 'dd' : 'dt',
        { 'data-placeholder': index ? tr('Définition') : tr('Terme') },
        0,
      ],
    }),
  ),
  ...(['postscript', 'attachment'] as const).map((name) =>
    Node.create({
      name,
      group: 'block',
      content: 'inline*',
      isolating: true,
      defining: true,
      parseHTML: () => [{ tag: `section[data-letter-note="${name}"]` }],
      renderHTML: () => ['section', { 'data-letter-note': name }, 0],
      addNodeView:
        () =>
        ({ getPos }) => {
          const frame = widgetFrame(name, getPos, properties, undefined, presentations[name])
          const { dom } = frame
          dom.dataset.letterNote = name
          const contentDOM = document.createElement('div')
          contentDOM.dataset.placeholder =
            name === 'postscript' ? tr('Texte du post-scriptum…') : tr('Document joint…')
          dom.append(contentDOM)
          return {
            dom,
            ...frame.content(contentDOM),
          }
        },
    }),
  ),
]
export function description(): JSONContent {
  return {
    type: 'descriptionList',
    content: [
      {
        type: 'descriptionItem',
        content: [{ type: 'descriptionTerm' }, { type: 'descriptionDefinition' }],
      },
    ],
  }
}
export function widgetTarget(editor: Editor) {
  const { $from } = editor.state.selection
  for (let depth = 1; depth <= $from.depth; depth++)
    if (isWidgetKind($from.node(depth).type.name))
      return { pos: $from.before(depth), node: $from.node(depth) }
  const node = editor.state.doc.nodeAt(editor.state.selection.from)
  if (node && isWidgetKind(node.type.name)) return { pos: editor.state.selection.from, node }
}
export function removeWidget(editor: Editor, target = widgetTarget(editor)) {
  if (
    !target ||
    !isWidgetKind(target.node.type.name) ||
    !editor.state.doc.nodeAt(target.pos)?.eq(target.node)
  )
    return false
  const manager = yUndoPluginKey.getState(editor.state)?.undoManager
  manager?.stopCapturing()
  const removed = editor.commands.deleteRange({
    from: target.pos,
    to: target.pos + target.node.nodeSize,
  })
  manager?.stopCapturing()
  return removed
}
export function resumeAfterWidget(tr: Transaction, pos: number) {
  const next = tr.doc.nodeAt(pos)
  if (next?.type.name !== 'paragraph' || next.content.size)
    tr.insert(pos, tr.doc.type.schema.nodes.paragraph.create())
  tr.setSelection(TextSelection.create(tr.doc, pos + 1))
  return tr
}
/** One annotation address for image captions, table captions and quote attributions. */
export function widgetAnnotation(target: { node: PMNode; pos: number }) {
  if (target.node.type.name === 'figure') return { kind: 'figure', position: target.pos + 1 }
  if (['quote', 'tableFigure'].includes(target.node.type.name))
    return {
      kind: target.node.lastChild!.type.name,
      position: target.pos + target.node.nodeSize - target.node.lastChild!.nodeSize,
    }
}
export function widgetKey(editor: Editor, event: KeyboardEvent): boolean {
  if (event.isComposing || editor.view.composing) return false
  const { $from } = editor.state.selection
  if ((event.ctrlKey || event.metaKey) && event.key === 'Backspace') {
    const target = widgetTarget(editor)
    if (!target) return false
    event.preventDefault()
    return removeWidget(editor)
  }
  if (event.key === 'Tab') {
    const target = widgetTarget(editor)
    if (target && ['quote', 'figure', 'tableFigure'].includes(target.node.type.name)) {
      if (target.node.type.name === 'tableFigure') {
        const cursor = tableCursor(editor)
        const inCaption = cursor.parent.type.name === 'tableCaption'
        const grid = target.node.firstChild!
        let row = -1,
          column = -1
        for (let depth = 1; depth <= cursor.depth; depth++) {
          if (cursor.node(depth).type.name === 'tableRow') row = cursor.index(depth - 1)
          if (['tableCell', 'tableHeader'].includes(cursor.node(depth).type.name))
            column = cursor.index(depth - 1)
        }
        if (
          !inCaption &&
          (event.shiftKey ||
            row !== grid.childCount - 1 ||
            column !== grid.lastChild!.childCount - 1)
        )
          return false
        if (inCaption && event.shiftKey) {
          event.preventDefault()
          return editor.commands.setTextSelection(
            TextSelection.near(editor.state.doc.resolve(target.pos + 1 + grid.nodeSize), -1).from,
          )
        }
      }
      event.preventDefault()
      const annotation = widgetAnnotation(target)!.position
      if (editor.state.selection.from < annotation) {
        return editor.commands.setTextSelection(annotation)
      }
      if (event.shiftKey && target.node.type.name === 'quote') {
        return editor.commands.setTextSelection(target.pos + 2)
      }
      const end = target.pos + target.node.nodeSize
      editor.view.dispatch(resumeAfterWidget(editor.state.tr, end).scrollIntoView())
      return true
    }
  }
  if (
    event.key === 'Tab' &&
    widgetTarget(editor)?.node.type.name.match(/^(bulletList|orderedList)$/)
  ) {
    event.preventDefault()
    return true
  }
  if (
    event.key === 'Tab' &&
    ['descriptionTerm', 'descriptionDefinition'].includes($from.parent.type.name)
  ) {
    event.preventDefault()
    const item = $from.node($from.depth - 1),
      start = $from.before($from.depth - 1)
    const term = item.child(0)
    return editor.commands.setTextSelection(event.shiftKey ? start + 2 : start + term.nodeSize + 2)
  }
  if (
    event.key === 'Enter' &&
    ['descriptionTerm', 'descriptionDefinition'].includes($from.parent.type.name)
  ) {
    event.preventDefault()
    const depth = $from.depth - 1,
      itemEnd = $from.after(depth)
    if ($from.parent.type.name === 'descriptionTerm')
      return editor.commands.setTextSelection(
        $from.before(depth) + $from.node(depth).child(0).nodeSize + 2,
      )
    if (!$from.sameParent(editor.state.selection.$to)) return false
    if (!$from.node(depth).textContent) {
      const listStart = $from.before(depth - 1),
        listEnd = $from.after(depth - 1)
      const only = $from.node(depth - 1).childCount === 1
      const tr = editor.state.tr.delete(
        only ? listStart : $from.before(depth),
        only ? listEnd : itemEnd,
      )
      const pos = only ? listStart : tr.mapping.map(listEnd)
      editor.view.dispatch(resumeAfterWidget(tr, pos).scrollIntoView())
      return true
    }
    const right = $from.parent.content.cut(editor.state.selection.$to.parentOffset)
    const next = editor.schema.nodes.descriptionItem.create(null, [
      editor.schema.nodes.descriptionTerm.create(),
      editor.schema.nodes.descriptionDefinition.create(null, right),
    ])
    const tr = editor.state.tr.delete(editor.state.selection.from, $from.end())
    const pos = tr.mapping.map(itemEnd)
    tr.insert(pos, next)
    tr.setSelection(TextSelection.create(tr.doc, pos + 2))
    editor.view.dispatch(tr)
    return true
  }
  return false
}

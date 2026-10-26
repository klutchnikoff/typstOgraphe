import { tr } from './i18n'
import { Node, type Editor, type JSONContent } from '@tiptap/core'
import { BulletList, OrderedList } from '@tiptap/extension-list'
import { icon } from './icons'
import type { Node as PMNode } from '@tiptap/pm/model'
import { TextSelection } from '@tiptap/pm/state'
export const widgetKinds = new Set([
  'figure',
  'quote',
  'descriptionList',
  'bulletList',
  'orderedList',
  'postscript',
  'attachment',
  'table',
])
export function widgetBar(
  kind: string,
  label: string,
  getPos: () => number | undefined,
  properties: (pos: number, kind: string) => void,
) {
  const bar = document.createElement('div')
  bar.className = 'widget-bar'
  bar.contentEditable = 'false'
  const title = document.createElement('span')
  title.textContent = label
  bar.append(title)
  for (const [name, iconName] of [
    [tr('Supprimer'), 'trash-2'],
    [tr('Paramètres'), 'settings-2'],
  ]) {
    const button = document.createElement('button')
    button.type = 'button'
    button.title = `${name} · ${label}`
    button.setAttribute('aria-label', `${name} · ${label}`)
    button.innerHTML = icon(iconName)
    button.onclick = () => {
      const pos = getPos()
      if (pos !== undefined) properties(pos, kind)
      if (name === tr('Supprimer'))
        document.querySelector<HTMLButtonElement>('#remove-block')?.focus()
    }
    bar.append(button)
  }
  return bar
}
export function widgetView(
  kind: string,
  label: string,
  tag: string,
  properties: (pos: number, kind: string) => void,
) {
  return ({ getPos, node }: { getPos: () => number | undefined; node: PMNode }) => {
    const dom = document.createElement('section')
    dom.className = `content-widget widget-${kind}`
    const bar = widgetBar(kind, label, getPos, properties)
    const contentDOM = document.createElement(tag)
    const update = (next: PMNode) => {
      if (next.type.name !== kind) return false
      if (kind === 'orderedList') contentDOM.setAttribute('start', String(next.attrs.start))
      return true
    }
    update(node)
    dom.append(bar, contentDOM)
    return { dom, contentDOM, update }
  }
}
export const simpleWidgets = (properties: (pos: number, kind: string) => void) => [
  ...([BulletList, OrderedList] as const).map((extension) =>
    extension.extend({
      addKeyboardShortcuts: () => ({}), // Les raccourcis publics viennent du registre.
      addAttributes() {
        const attributes = this.parent?.() || {}
        // Le style de numérotation appartient au modèle, pas au contenu éditable.
        delete attributes.type
        return attributes
      },
      addNodeView: () =>
        widgetView(
          extension.name,
          extension.name === 'orderedList' ? tr('Liste numérotée') : tr('Liste à puces'),
          extension.name === 'orderedList' ? 'ol' : 'ul',
          properties,
        ),
    }),
  ),
  Node.create({
    name: 'quote',
    addNodeView: () => widgetView('quote', tr('Citation'), 'blockquote', properties),
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
    addNodeView: () => widgetView('descriptionList', tr('Descriptions'), 'dl', properties),
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
          const dom = document.createElement('section')
          dom.dataset.letterNote = name
          const bar = widgetBar(name, name === 'postscript' ? 'P.-S.' : 'P.J.', getPos, properties)
          const contentDOM = document.createElement('div')
          contentDOM.dataset.placeholder =
            name === 'postscript' ? tr('Texte du post-scriptum…') : tr('Document joint…')
          dom.append(bar, contentDOM)
          return { dom, contentDOM }
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
    if (widgetKinds.has($from.node(depth).type.name))
      return { pos: $from.before(depth), node: $from.node(depth) }
  const node = editor.state.doc.nodeAt(editor.state.selection.from)
  if (node && widgetKinds.has(node.type.name)) return { pos: editor.state.selection.from, node }
}
export function removeWidget(editor: Editor) {
  const target = widgetTarget(editor)
  return (
    !!target &&
    editor.commands.deleteRange({ from: target.pos, to: target.pos + target.node.nodeSize })
  )
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
    if (target && ['quote', 'figure'].includes(target.node.type.name)) {
      event.preventDefault()
      const annotation =
        target.node.type.name === 'figure'
          ? target.pos + 1
          : target.pos + target.node.nodeSize - target.node.lastChild!.nodeSize
      if (editor.state.selection.from < annotation) {
        return editor.commands.setTextSelection(annotation)
      }
      if (event.shiftKey && target.node.type.name === 'quote') {
        return editor.commands.setTextSelection(target.pos + 2)
      }
      const end = target.pos + target.node.nodeSize
      if (editor.state.doc.nodeAt(end)?.isTextblock)
        return editor.commands.setTextSelection(end + 1)
      return editor
        .chain()
        .insertContentAt(end, { type: 'paragraph' })
        .setTextSelection(end + 1)
        .run()
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
      tr.insert(pos, editor.schema.nodes.paragraph.create())
      tr.setSelection(TextSelection.create(tr.doc, pos + 1))
      editor.view.dispatch(tr)
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

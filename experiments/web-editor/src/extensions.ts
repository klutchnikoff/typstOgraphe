import {
  Extension,
  Node,
  mergeAttributes,
  markInputRule,
  textblockTypeInputRule,
  type Extensions,
} from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { ListItem } from '@tiptap/extension-list'
import { TableKit, Table, TableView } from '@tiptap/extension-table'
import Collaboration from '@tiptap/extension-collaboration'
import Caret from '@tiptap/extension-collaboration-caret'
import type { Provider } from './provider'
import { simpleWidgets } from './widgets'
import { widgetFrame, type WidgetIcons } from './widget-frame'
import { tr } from './i18n'
import { Proofing } from './proofing'
import { DocumentInput } from './editing'
import { ContextBand } from './band-layout'

export function baseExtensions(
  imageURL: (path: string) => string,
  properties: (pos: number, kind: string, remove?: boolean) => void = () => {},
  icons: WidgetIcons = {},
): Extensions {
  return [
    Proofing,
    ContextBand,
    DocumentInput,
    ListItem.extend({ content: 'paragraph+' }),
    ...simpleWidgets(properties, icons),
    // Comme dans l'éditeur existant : une fin de ligne reste dans le paragraphe.
    Node.create({
      name: 'softBreak',
      group: 'inline',
      inline: true,
      selectable: false,
      parseHTML: () => [{ tag: 'br[data-soft-break]' }],
      renderHTML: () => ['br', { 'data-soft-break': '' }],
    }),
    Extension.create({
      name: 'paragraphInput',
      priority: 1000,
      addInputRules() {
        return [
          ...[1, 2, 3].map((level) =>
            textblockTypeInputRule({
              find: new RegExp(`^={${level}}\\s$`),
              type: this.editor.schema.nodes.heading,
              getAttributes: { level },
            }),
          ),
          markInputRule({ find: /(?:^|\s)(\*([^*]+)\*)$/, type: this.editor.schema.marks.bold }),
        ]
      },
    }),
    StarterKit.configure({
      undoRedo: false,
      trailingNode: false,
      bulletList: false,
      listItem: false,
      orderedList: false,
      heading: { levels: [1, 2, 3] },
      blockquote: false,
      code: false,
      codeBlock: false,
      strike: false,
      horizontalRule: false,
      link: { openOnClick: false },
    }),
    TableKit.configure({ table: false }),
    Table.extend({
      addNodeView() {
        return ({ node, view, getPos }) => {
          const table = new TableView(node, this.options.cellMinWidth, view)
          const frame = widgetFrame('table', getPos, properties, table.dom, icons.table)
          table.ignoreMutation = frame.content(
            table.contentDOM,
            table.ignoreMutation.bind(table),
          ).ignoreMutation
          return table
        }
      },
    }).configure({ resizable: false }),
    Node.create({
      name: 'footnote',
      group: 'inline',
      inline: true,
      atom: true,
      selectable: true,
      addAttributes: () => ({ text: { default: '' } }),
      parseHTML: () => [
        {
          tag: 'span[data-footnote]',
          getAttrs: (element) => ({ text: element.getAttribute('data-footnote') }),
        },
      ],
      renderHTML: ({ HTMLAttributes }) => [
        'span',
        mergeAttributes(HTMLAttributes, {
          'data-footnote': HTMLAttributes.text,
          class: 'footnote-ref',
        }),
        '※',
      ],
      addNodeView:
        () =>
        ({ node, getPos }) => {
          const dom = document.createElement('sup')
          dom.className = 'footnote-ref'
          dom.textContent = '※'
          dom.title = node.attrs.text || 'Note de bas de page'
          dom.setAttribute('role', 'button')
          dom.setAttribute('aria-label', `Note : ${node.attrs.text || 'à renseigner'}`)
          dom.tabIndex = 0
          const open = () => {
            const pos = getPos()
            if (pos !== undefined) properties(pos, 'footnote')
          }
          dom.addEventListener('click', open)
          dom.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              open()
            }
          })
          return {
            dom,
            update(next) {
              if (next.type.name !== 'footnote') return false
              dom.title = next.attrs.text
              dom.setAttribute('aria-label', `Note : ${next.attrs.text}`)
              return true
            },
          }
        },
    }),
    Node.create({
      name: 'figure',
      group: 'block',
      content: 'inline*',
      isolating: true,
      defining: true,
      addAttributes: () => ({ src: { default: '' }, alt: { default: '' } }),
      parseHTML: () => [
        {
          tag: 'figure[data-image]',
          contentElement: 'figcaption',
          getAttrs: (element) => ({
            src: element.getAttribute('data-image'),
            alt: element.querySelector('img')?.alt || '',
          }),
        },
      ],
      renderHTML: ({ node, HTMLAttributes }) => [
        'figure',
        { 'data-image': node.attrs.src },
        ['img', { src: imageURL(node.attrs.src), alt: HTMLAttributes.alt }],
        ['figcaption', 0],
      ],
      addNodeView:
        () =>
        ({ node, getPos }) => {
          const frame = widgetFrame(
            'figure',
            getPos,
            properties,
            document.createElement('figure'),
            icons.figure,
          )
          const { dom } = frame
          const image = document.createElement('img')
          image.src = imageURL(node.attrs.src)
          image.alt = node.attrs.alt
          image.contentEditable = 'false'
          image.draggable = false
          const caption = document.createElement('figcaption')
          caption.dataset.placeholder = tr('Légende facultative')
          dom.append(image, caption)
          return {
            dom,
            ...frame.content(caption),
            update(next) {
              if (next.type.name !== 'figure') return false
              image.src = imageURL(next.attrs.src)
              image.alt = next.attrs.alt
              return true
            },
          }
        },
    }),
  ]
}

export async function collaborativeExtensions(
  provider: Provider,
  properties: (pos: number, kind: string, remove?: boolean) => void,
  icons: WidgetIcons = {},
): Promise<Extensions> {
  return [
    ...baseExtensions((path) => provider.imageURL(path), properties, icons),
    Collaboration.configure({ document: provider.doc, field: 'default' }),
    Caret.configure({
      provider,
      user: {
        name: provider.id === 'main' ? 'Camille' : 'Alex',
        color: provider.id === 'main' ? '#286b91' : '#99733b',
      },
    }),
  ]
}

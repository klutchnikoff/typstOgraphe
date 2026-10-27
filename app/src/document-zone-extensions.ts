import {
  Extension,
  Node,
  mergeAttributes,
  textblockTypeInputRule,
  type Extensions,
} from '@tiptap/core'
import { ListItem } from '@tiptap/extension-list'
import { TableKit, Table, TableView } from '@tiptap/extension-table'
import { simpleWidgets } from './widgets'
import { widgetFrame, type WidgetPresentations } from './widget-frame'
import { tr } from './i18n'
import { Proofing } from './proofing'
import { inlineObjectView } from './inline-objects'
import { InlineObjectNumbering, inlineObjectNumber } from './inline-object-numbering'
import { citationLabel } from './citation-index'
import { InlineObjectFocus } from './inline-object-focus'

export function documentExtensions(
  imageURL: (path: string) => string,
  properties: (pos: number, kind: string, remove?: boolean) => void = () => {},
  presentations: WidgetPresentations = {},
): Extensions {
  const extensions: Extensions = [
    Proofing,
    InlineObjectNumbering.configure({ kinds: ['footnote'] }),
    InlineObjectFocus,
    ListItem.extend({ content: 'paragraph+' }),
    ...simpleWidgets(properties, presentations),
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
        ]
      },
    }),
    TableKit.configure({ table: false }),
    Table.extend({
      addNodeView() {
        return ({ node, view, getPos }) => {
          const table = new TableView(node, this.options.cellMinWidth, view)
          const frame = widgetFrame('table', getPos, properties, table.dom, presentations.table)
          table.ignoreMutation = frame.content(
            table.contentDOM,
            table.ignoreMutation.bind(table),
          ).ignoreMutation
          return table
        }
      },
    }).configure({ resizable: false }),
    Node.create({
      name: 'citation',
      group: 'inline',
      inline: true,
      atom: true,
      selectable: true,
      addAttributes: () => ({ items: { default: [] }, form: { default: 'normal' } }),
      parseHTML: () => [
        {
          tag: 'span[data-citation]',
          getAttrs: (element) => ({
            items: JSON.parse(element.getAttribute('data-citation') || '[]'),
            form: element.getAttribute('data-form') || 'normal',
          }),
        },
      ],
      renderHTML: ({ node }) => [
        'span',
        { 'data-citation': JSON.stringify(node.attrs.items), 'data-form': node.attrs.form },
        node.attrs.items.map((item: { key: string }) => item.key).join('; '),
      ],
      addNodeView:
        () =>
        ({ node, getPos, editor }) => {
          let current = node
          const badge = inlineObjectView({
            editor,
            getPos,
            kind: 'citation',
            className: 'citation-ref',
            label: tr('Citation'),
            title: tr('Citation bibliographique'),
            open: properties,
          })
          const updateLabel = () =>
            badge.describe(
              current.attrs.items.map((item: { key: string }) => item.key).join('; '),
              undefined,
              citationLabel(editor, current.attrs.items),
            )
          const transaction = ({
            transaction,
          }: {
            transaction: import('@tiptap/pm/state').Transaction
          }) => {
            if (transaction.getMeta('bibliography-index')) updateLabel()
          }
          editor.on('transaction', transaction)
          updateLabel()
          return {
            dom: badge.dom,
            update(next) {
              if (next.type.name !== 'citation') return false
              current = next
              updateLabel()
              return true
            },
            destroy() {
              editor.off('transaction', transaction)
            },
          }
        },
    }),
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
        tr('Note'),
      ],
      addNodeView:
        () =>
        ({ node, getPos, editor, decorations }) => {
          const badge = inlineObjectView({
            editor,
            getPos,
            kind: 'footnote',
            className: 'footnote-ref',
            label: tr('Note'),
            title: tr('Note de bas de page'),
            open: properties,
          })
          badge.describe(node.attrs.text, inlineObjectNumber(decorations))
          return {
            dom: badge.dom,
            update(next, decorations) {
              if (next.type.name !== 'footnote') return false
              badge.describe(next.attrs.text, inlineObjectNumber(decorations))
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
            presentations.figure,
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
  return extensions.map((extension) => {
    const fields = presentations[extension.name as keyof WidgetPresentations]?.settings.fields
    if (extension.type !== 'node' || !fields?.length) return extension
    return (extension as Node).extend({
      addAttributes() {
        const existing = this.parent?.() || {}
        const declared = Object.fromEntries(
          fields.map((field) => [field.attribute, { default: field.schema.default ?? null }]),
        )
        return { ...declared, ...existing }
      },
    })
  })
}

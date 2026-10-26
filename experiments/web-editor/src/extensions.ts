import { Extension, Node, mergeAttributes, type Extensions } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { TableKit } from '@tiptap/extension-table'
import Collaboration from '@tiptap/extension-collaboration'
import Caret from '@tiptap/extension-collaboration-caret'
import type { Provider } from './provider'

export function baseExtensions(
  imageURL: (path: string) => string,
  properties: (pos: number, kind: string) => void = () => {},
): Extensions {
  return [
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
      addKeyboardShortcuts() {
        return {
          Enter: () => {
            const { selection } = this.editor.state
            const { $from } = selection
            // Titres, cellules, listes et légendes gardent leur navigation structurelle.
            if ($from.parent.type.name !== 'paragraph' || $from.depth !== 1) return false
            if (selection.empty && $from.nodeBefore?.type.name === 'softBreak') {
              return this.editor
                .chain()
                .deleteRange({ from: $from.pos - 1, to: $from.pos })
                .splitBlock()
                .run()
            }
            return this.editor.commands.insertContent({ type: 'softBreak' })
          },
        }
      },
    }),
    StarterKit.configure({
      undoRedo: false,
      heading: { levels: [1, 2, 3] },
      blockquote: false,
      code: false,
      codeBlock: false,
      strike: false,
      horizontalRule: false,
      link: { openOnClick: false },
    }),
    TableKit.configure({ table: { resizable: false } }),
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
          const dom = document.createElement('figure')
          const bar = document.createElement('div')
          bar.className = 'figure-bar'
          bar.contentEditable = 'false'
          const label = document.createElement('span')
          label.textContent = 'Image'
          const settings = document.createElement('button')
          settings.type = 'button'
          settings.textContent = 'Paramètres'
          settings.setAttribute('aria-label', 'Paramètres de l’image')
          settings.addEventListener('click', () => {
            const pos = getPos()
            if (pos !== undefined) properties(pos, 'figure')
          })
          bar.append(label, settings)
          const image = document.createElement('img')
          image.src = imageURL(node.attrs.src)
          image.alt = node.attrs.alt
          image.contentEditable = 'false'
          image.draggable = false
          const caption = document.createElement('figcaption')
          caption.dataset.placeholder = 'Légende facultative'
          dom.append(bar, image, caption)
          return {
            dom,
            contentDOM: caption,
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
  properties: (pos: number, kind: string) => void,
): Promise<Extensions> {
  return [
    ...baseExtensions((path) => provider.imageURL(path), properties),
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

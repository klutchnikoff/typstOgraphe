import { Extension, Node, type Extensions } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { ContextBand } from './band-layout'
import { DocumentInput, SoftBreak, TypstInlineInput } from './editing'

/** The same restricted schema seeds, reads and edits named rich metadata zones. */
export function editorialExtensions(inline: boolean): Extensions {
  return [
    Node.create({ name: 'doc', topNode: true, content: inline ? 'paragraph' : 'paragraph+' }),
    ContextBand,
    SoftBreak,
    TypstInlineInput,
    ...(!inline ? [DocumentInput] : []),
    Extension.create({
      name: 'editorialLine',
      addKeyboardShortcuts() {
        return { Enter: () => inline, 'Shift-Enter': () => inline }
      },
    }),
    StarterKit.configure({
      document: false,
      undoRedo: false,
      trailingNode: false,
      heading: false,
      blockquote: false,
      bulletList: false,
      orderedList: false,
      listItem: false,
      code: false,
      codeBlock: false,
      strike: false,
      horizontalRule: false,
      link: { openOnClick: false },
    }),
  ]
}

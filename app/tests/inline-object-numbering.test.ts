import { afterEach, expect, it } from 'vitest'
import { Editor, type JSONContent } from '@tiptap/core'
import { baseExtensions } from '../src/extensions'
import { inlineObjectNumberingKey } from '../src/inline-object-numbering'

const editors: Editor[] = []
afterEach(() => {
  editors.splice(0).forEach((editor) => editor.destroy())
  document.body.replaceChildren()
})
const note = (text: string): JSONContent => ({ type: 'footnote', attrs: { text } })
const paragraph = (text: string): JSONContent => ({
  type: 'paragraph',
  content: [{ type: 'text', text: 'Texte' }, note(text)],
})
function create() {
  const element = document.createElement('div')
  document.body.append(element)
  const editor = new Editor({
    element,
    extensions: baseExtensions(() => ''),
    content: {
      type: 'doc',
      content: [
        paragraph('A'),
        { type: 'quote', content: [paragraph('B'), { type: 'quoteAttribution' }] },
        {
          type: 'table',
          content: [
            { type: 'tableRow', content: [{ type: 'tableCell', content: [paragraph('C')] }] },
          ],
        },
      ],
    },
  })
  editors.push(editor)
  editor.commands.setTextSelection(1)
  return editor
}
const badges = (editor: Editor) => [...editor.view.dom.querySelectorAll('.footnote-ref')]
const labels = (editor: Editor) => badges(editor).map((badge) => badge.textContent)

it('numérote dans l’ordre du document, y compris les citations et cellules, sans écrire d’attribut de numéro', () => {
  const editor = create()
  expect(labels(editor)).toEqual(['Note 1', 'Note 2', 'Note 3'])
  expect(badges(editor).map((badge) => badge.getAttribute('aria-label'))).toEqual([
    'Note de bas de page 1 : A',
    'Note de bas de page 2 : B',
    'Note de bas de page 3 : C',
  ])
  editor.state.doc.descendants((node) => {
    if (node.type.name === 'footnote') expect(Object.keys(node.attrs)).toEqual(['text'])
  })
  expect(editor.getHTML()).not.toContain('data-inline-number')
  const decorations = inlineObjectNumberingKey.getState(editor.state)
  editor.commands.setTextSelection(3)
  expect(inlineObjectNumberingKey.getState(editor.state)).toBe(decorations)
})
it('renumérote les vues existantes après insertion, suppression et déplacement', () => {
  const editor = create()
  const b = badges(editor)[1]
  editor.commands.insertContentAt(1, note('Ajout'))
  expect(labels(editor)).toEqual(['Note 1', 'Note 2', 'Note 3', 'Note 4'])
  expect(badges(editor)[2]).toBe(b)
  expect(b.textContent).toBe('Note 3')
  editor.commands.deleteRange({ from: 1, to: 2 })
  expect(labels(editor)).toEqual(['Note 1', 'Note 2', 'Note 3'])
  let last = 0
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name === 'footnote') last = pos
  })
  const moved = editor.state.doc.nodeAt(last)!
  editor.view.dispatch(editor.state.tr.delete(last, last + moved.nodeSize).insert(1, moved))
  expect(labels(editor)).toEqual(['Note 1', 'Note 2', 'Note 3'])
  expect(badges(editor).map((badge) => badge.getAttribute('title'))).toEqual([
    'Note de bas de page 1 : C',
    'Note de bas de page 2 : A',
    'Note de bas de page 3 : B',
  ])
})

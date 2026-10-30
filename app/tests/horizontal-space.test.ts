import { Editor, getSchema } from '@tiptap/core'
import { prosemirrorJSONToYXmlFragment } from '@tiptap/y-tiptap'
import { afterEach, expect, it } from 'vitest'
import * as Y from 'yjs'
import { richZoneExtensions, type RichZoneProfile } from '../src/rich-zone'
import { normalizeSharedSpaces } from '../src/horizontal-space'
const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((e) => e.destroy()))
function create(profile: RichZoneProfile, content = '<p>Deux mots</p>') {
  const e = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions(profile),
    content,
  })
  editors.push(e)
  e.commands.setTextSelection(5)
  return e
}
function input(e: Editor, text = ' ') {
  const { from, to } = e.state.selection
  return e.view.someProp('handleTextInput', (h) =>
    h(e.view, from, to, text, () => e.state.tr.insertText(text, from, to)),
  )
}
it.each(['document', 'rich-text', 'inline-text'] as const)(
  'fait cycler une seule espace sans perdre le curseur dans %s',
  (profile) => {
    const e = create(profile)
    // Start with no existing space, then cycle the same gap, including in mid-text.
    e.commands.deleteRange({ from: 5, to: 6 })
    for (const space of [' ', '\u00a0', ' ', '\u00a0']) {
      expect(input(e)).toBe(true)
      expect(e.state.doc.firstChild!.textContent).toBe(`Deux${space}mots`)
      expect(e.state.selection.from).toBe(6)
    }
    expect(e.view.dom.querySelector('.nonbreaking-space')?.textContent).toBe('\u00a0')
    expect(e.view.dom.textContent).not.toContain('~')
    e.commands.undo()
    expect(e.state.doc.firstChild!.textContent).toBe('Deux mots')
  },
)
it('fonctionne avant une espace, conserve les styles explicites et remplace une sélection', () => {
  const e = create('document', '<p><strong>Deux mots</strong></p>')
  e.commands.setTextSelection(5)
  e.view.dispatch(e.state.tr.setStoredMarks([]))
  input(e)
  expect(e.state.doc.firstChild!.textContent).toBe('Deux\u00a0mots')
  expect(e.state.storedMarks).toEqual([])
  e.commands.setTextSelection({ from: 6, to: 10 })
  input(e)
  expect(e.state.doc.firstChild!.textContent).toBe('Deux ')
})
it('laisse les prééditions IME intactes et ne confond ni tilde littéral ni espace fine', () => {
  const e = create('document', '<p>A~B\u202fC</p>')
  e.commands.setTextSelection(2)
  Object.defineProperty(e.view, 'composing', { configurable: true, value: true })
  expect(input(e)).not.toBe(true)
  e.commands.setContent({
    type: 'doc',
    content: [{ type: 'paragraph', content: [{ type: 'text', text: 'A  B' }] }],
  })
  expect(e.state.doc.firstChild!.textContent).toBe('A  B')
  Object.defineProperty(e.view, 'composing', { configurable: true, value: false })
})
it.each(['document', 'rich-text', 'inline-text'] as const)(
  'réduit le collage et les espaces entre styles sans modifier les mots (%s)',
  (profile) => {
    const e = create(profile, '<p><strong>A😀 </strong><em>  Bé\u00a0\u00a0 fin</em></p>')
    expect(e.state.doc.firstChild!.textContent).toBe('A😀 Bé\u00a0fin')
    expect(e.state.doc.firstChild!.firstChild!.marks[0].type.name).toBe('bold')
    expect(e.state.doc.firstChild!.lastChild!.marks[0].type.name).toBe('italic')
  },
)
it('la réparation partagée travaille sur les caractères et non la sérialisation XML des styles', () => {
  const doc = new Y.Doc(),
    root = doc.getXmlFragment('default')
  const schema = getSchema(richZoneExtensions('document'))
  prosemirrorJSONToYXmlFragment(
    schema,
    {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'A😀  ', marks: [{ type: 'bold' }] },
            { type: 'text', text: '  Bé\u00a0\u00a0fin', marks: [{ type: 'italic' }] },
          ],
        },
      ],
    },
    root,
  )
  doc.transact(() => normalizeSharedSpaces(root))
  const text = (root.get(0) as Y.XmlElement).get(0) as Y.XmlText
  expect(
    text
      .toDelta()
      .map((p: { insert: unknown }) => p.insert)
      .join(''),
  ).toBe('A😀 Bé\u00a0fin')
  expect(text.toDelta()[0].attributes?.bold).toBeDefined()
  doc.destroy()
})

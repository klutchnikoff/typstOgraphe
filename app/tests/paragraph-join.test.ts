import { Editor } from '@tiptap/core'
import { TextSelection } from '@tiptap/pm/state'
import { afterEach, expect, it } from 'vitest'
import { richZoneExtensions, type RichZoneProfile } from '../src/rich-zone'
import { joinParagraphs } from '../src/paragraph-join'
const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
function create(content: string, profile: RichZoneProfile = 'document') {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions(profile),
    content,
  })
  editors.push(editor)
  editor.commands.setTextSelection(1)
  return editor
}
function press(editor: Editor, key: string) {
  return editor.view.someProp('handleKeyDown', (handler) =>
    handler(editor.view, new KeyboardEvent('keydown', { key })),
  )
}
it.each(['Backspace', 'Delete'])(
  'conserve un retour simple entre paragraphes, le type gauche et les marques (%s)',
  (key) => {
    for (const [a, b] of [
      ['p', 'p'],
      ['h1', 'p'],
      ['p', 'h2'],
      ['h2', 'h3'],
    ]) {
      const e = create(`<${a}><strong>A😀</strong></${a}><${b}><em>Bé</em></${b}>`)
      const left = e.state.doc.firstChild!
      e.commands.setTextSelection(key === 'Backspace' ? left.nodeSize + 1 : left.nodeSize - 1)
      expect(press(e, key)).toBe(true)
      expect(e.state.doc.firstChild!.type.name).toBe(a === 'p' ? 'paragraph' : 'heading')
      expect(e.state.doc.firstChild!.textContent).toBe('A😀Bé')
      expect(e.state.selection.from).toBe(a === 'p' && b === 'p' && key === 'Backspace' ? 5 : 4)
      expect(
        e.state.doc.firstChild!.content.content.filter((n) => n.type.name === 'softBreak'),
      ).toHaveLength(a === 'p' && b === 'p' ? 1 : 0)
      expect(e.state.doc.firstChild!.firstChild!.marks[0].type.name).toBe('bold')
      expect(e.state.doc.firstChild!.lastChild!.marks[0].type.name).toBe('italic')
      expect(e.state.storedMarks?.[0].type.name).toBe('bold')
    }
  },
)
it('conserve un arrêt explicite de style et laisse la suppression interne et les sélections au moteur normal', () => {
  const e = create('<p>A</p><p>B</p>')
  e.commands.setTextSelection(4)
  e.view.dispatch(e.state.tr.setStoredMarks([]))
  expect(press(e, 'Backspace')).toBe(true)
  expect(e.state.storedMarks).toEqual([])
  e.commands.setTextSelection(2)
  expect(joinParagraphs(e, true, true)).toBe(false)
  e.commands.setTextSelection({ from: 1, to: 2 })
  expect(joinParagraphs(e, true, true)).toBe(false)
})
it('retire le voisin vide sans convertir le titre, protège la reprise finale et les widgets', () => {
  const e = create(
    '<p>Avant</p><h1></h1><h2>Titre</h2><blockquote><p>Citation</p><div data-quote-attribution>Source</div></blockquote>',
  )
  e.commands.setTextSelection(10)
  expect(press(e, 'Backspace')).toBe(true)
  expect(e.state.doc.child(1).attrs.level).toBe(2)
  const before = e.getJSON()
  e.commands.setTextSelection(13)
  expect(press(e, 'Delete')).toBe(true)
  expect(e.getJSON()).toEqual(before)
  e.commands.setTextSelection(e.state.doc.content.size - 1)
  expect(press(e, 'Backspace')).toBe(true)
  expect(e.getJSON()).toEqual(before)
})
it.each(['document', 'rich-text'] as const)(
  'joint dans une zone %s et annule/rétablit sans altérer les blocs',
  (profile) => {
    const e = create('<p>Avant</p><p>Après</p>', profile)
    const before = e.getJSON()
    e.commands.setTextSelection(8)
    expect(press(e, 'Backspace')).toBe(true)
    expect(e.state.selection.from).toBe(7)
    e.commands.undo()
    expect(e.getJSON()).toEqual(before)
    expect(e.state.selection.from).toBe(8)
    e.commands.redo()
    expect(e.state.doc.firstChild!.textContent).toBe('AvantAprès')
    expect(e.state.selection.from).toBe(7)
  },
)
it('joint au sein d’une citation, protège son attribution et les cellules voisines', () => {
  const e = create(
    '<blockquote><p>A</p><p>B</p></blockquote><table><tr><td><p>C</p><p>D</p></td><td>E</td></tr></table>',
  )
  let positions: number[] = []
  e.state.doc.descendants((n, pos) => {
    if (n.type.name === 'paragraph' && n.textContent) positions.push(pos + 1)
  })
  e.commands.setTextSelection(positions[1])
  expect(press(e, 'Backspace')).toBe(true)
  expect(e.state.doc.firstChild!.firstChild!.textContent).toBe('AB')
  positions = []
  e.state.doc.descendants((n, pos) => {
    if (n.type.name === 'paragraph' && n.textContent) positions.push(pos + 1)
  })
  e.commands.setTextSelection(positions[2])
  expect(press(e, 'Backspace')).toBe(true)
  const before = e.getJSON()
  e.state.doc.descendants((n, pos) => {
    if (n.type.name === 'paragraph' && n.textContent === 'E') e.commands.setTextSelection(pos + 1)
  })
  expect(press(e, 'Backspace')).toBe(true)
  expect(e.getJSON()).toEqual(before)
})

it.each(['Backspace', 'Delete'])(
  'transforme PP en NN et laisse sa suppression suivante au comportement en ligne (%s)',
  (key) => {
    const e = create('<p>Un texte.</p><p>Un autre texte.</p>')
    e.commands.setTextSelection(key === 'Backspace' ? 12 : 10)
    expect(press(e, key)).toBe(true)
    expect(e.state.doc.firstChild!.textContent).toBe('Un texte.Un autre texte.')
    expect(e.state.doc.firstChild!.child(1).type.name).toBe('softBreak')
    expect(e.state.selection.from).toBe(key === 'Backspace' ? 11 : 10)
    expect(joinParagraphs(e, key === 'Backspace', true)).toBe(false)
  },
)

it.each(['Backspace', 'Delete'])(
  'réutilise un retour existant de chaque côté sans les doubler (%s)',
  (key) => {
    for (const [left, right] of [
      ['A<br data-soft-break>', 'B'],
      ['A', '<br data-soft-break>B'],
      ['A<br data-soft-break>', '<br data-soft-break>B'],
    ]) {
      const e = create(`<p>${left}</p><p>${right}</p>`)
      const size = e.state.doc.firstChild!.nodeSize
      e.commands.setTextSelection(key === 'Backspace' ? size + 1 : size - 1)
      expect(press(e, key)).toBe(true)
      expect(e.state.doc.firstChild!.content.content.map((n) => n.type.name)).toEqual([
        'text',
        'softBreak',
        'text',
      ])
      expect(e.state.selection.from).toBe(key === 'Backspace' ? 3 : 2)
    }
  },
)

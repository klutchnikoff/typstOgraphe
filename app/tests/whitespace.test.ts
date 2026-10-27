import { afterEach, expect, it } from 'vitest'
import { Editor } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { documentWhitespaceKey, normalizeWhitespace } from '../src/document-whitespace'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
function create(html: string) {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions('document'),
    content: html,
  })
  editors.push(editor)
  editor.commands.setTextSelection(1)
  return editor
}
const children = (editor: Editor) => editor.state.doc.content.content

it.each(['Enter', 'Shift-Enter'])(
  '%s réutilise le retour déjà présent devant le curseur',
  (key) => {
    const editor = create('<p>Avant<br data-soft-break>Après</p>')
    editor.commands.setTextSelection(6)
    const before = editor.getJSON()
    const event = new KeyboardEvent('keydown', {
      key: 'Enter',
      shiftKey: key === 'Shift-Enter',
      bubbles: true,
      cancelable: true,
    })
    editor.view.someProp('handleKeyDown', (handler) => handler(editor.view, event))
    expect(editor.getJSON()).toEqual(before)
    expect(editor.state.selection.from).toBe(7)
    editor.commands.insertContent('suite ')
    expect(editor.state.doc.firstChild!.textContent).toBe('Avantsuite Après')
  },
)
it('un retour simple reste dans le paragraphe et le second crée le suivant sans double blanc final', () => {
  const editor = create('<p>Avant</p>')
  expect(children(editor).map((p) => p.textContent)).toEqual(['Avant', ''])
  editor.commands.setTextSelection(6)
  editor.commands.keyboardShortcut('Enter')
  expect(children(editor)).toHaveLength(1)
  expect(editor.state.doc.lastChild!.lastChild!.type.name).toBe('softBreak')
  editor.commands.keyboardShortcut('Enter')
  expect(children(editor).map((p) => p.textContent)).toEqual(['Avant', ''])
  for (let i = 0; i < 4; i++) editor.commands.keyboardShortcut('Enter')
  expect(children(editor).map((p) => p.textContent)).toEqual(['Avant', ''])
})
it('le collage réduit les retours répétés et les paragraphes vides sans toucher aux marques', () => {
  const editor = create('<p></p>')
  editor.commands.setContent(
    '<p><strong>Avant</strong><br data-soft-break><br data-soft-break><br>Après</p><p></p><p></p><p>Suite</p>',
  )
  expect(children(editor).map((p) => p.textContent)).toEqual(['AvantAprès', '', 'Suite', ''])
  const content = editor.state.doc.firstChild!.content.content
  expect(content.map((node) => node.type.name)).toEqual(['text', 'softBreak', 'text'])
  expect(content[0].marks[0].type.name).toBe('bold')
  expect(normalizeWhitespace(editor.state.tr).docChanged).toBe(false)
})
it('la dernière ligne est recréée après saisie et suppression, sans déplacer le curseur', () => {
  const editor = create('<p>Avant</p>')
  editor.commands.setTextSelection(editor.state.doc.content.size - 1)
  editor.commands.insertContent('Suite')
  expect(children(editor).map((p) => p.textContent)).toEqual(['Avant', 'Suite', ''])
  expect(editor.state.selection.$from.parent.textContent).toBe('Suite')
  editor.commands.deleteRange({
    from: editor.state.doc.content.size - 2,
    to: editor.state.doc.content.size,
  })
  expect(children(editor).map((p) => p.textContent)).toEqual(['Avant', 'Suite', ''])
  expect(editor.state.selection.$from.parent.textContent).toBe('Suite')
})
it('ne réécrit pas les espaces du document pendant la composition IME', () => {
  const editor = create('<p></p>')
  Object.defineProperty(editor.view, 'composing', { configurable: true, value: true })
  editor.commands.setContent('<p>にほん</p><p></p><p></p>')
  expect(children(editor)).toHaveLength(3)
  Object.defineProperty(editor.view, 'composing', { configurable: true, value: false })
  editor.view.dispatch(editor.state.tr.setMeta(documentWhitespaceKey, true))
  expect(children(editor).map((p) => p.textContent)).toEqual(['にほん', ''])
})

it('réutilise aussi la frontière du paragraphe suivant sans créer de ligne vide intermédiaire', () => {
  const editor = create('<p>Avant</p><p>Après</p>')
  editor.commands.setTextSelection(6)
  const before = editor.getJSON()
  editor.view.someProp('handleKeyDown', (handler) =>
    handler(editor.view, new KeyboardEvent('keydown', { key: 'Enter' })),
  )
  expect(editor.getJSON()).toEqual(before)
  expect(editor.state.selection.from).toBe(8)
})

it.each(['Enter', 'Shift-Enter'])(
  '%s ne crée aucune ligne vide initiale ni modification inutile',
  (key) => {
    for (const html of ['<p>Premier</p>', '<h1>Titre</h1>', '<p></p>', '<h1></h1>']) {
      const editor = create(html),
        before = editor.getJSON()
      let updates = 0
      editor.on('update', () => updates++)
      editor.commands.keyboardShortcut(key)
      expect(editor.getJSON()).toEqual(before)
      expect(editor.state.selection.from).toBe(1)
      expect(updates).toBe(0)
      editor.commands.insertContent('Début ')
      expect(editor.state.doc.firstChild!.textContent).toMatch(/^Début /)
    }
  },
)
it('supprime les lignes initiales importées en préservant les marques et la dernière ligne', () => {
  const editor = create('<p></p>')
  editor.commands.setContent(
    '<p></p><h2></h2><p><br data-soft-break><br><strong>Corps</strong></p>',
  )
  expect(children(editor).map((node) => node.textContent)).toEqual(['Corps', ''])
  expect(editor.state.doc.firstChild!.firstChild!.marks[0].type.name).toBe('bold')
  expect(normalizeWhitespace(editor.state.tr).docChanged).toBe(false)
})
it('préserve le titre vide choisi avant la saisie et les retours internes aux widgets', () => {
  const editor = create('<h1></h1>')
  expect(editor.state.doc.firstChild!.type.name).toBe('heading')
  editor.commands.setContent('<blockquote><p><br data-soft-break>Interne</p></blockquote>')
  // Root normalization must not consume the first line inside a widget.
  expect(editor.state.doc.firstChild!.type.name).toBe('quote')
  expect(editor.state.doc.firstChild!.firstChild!.firstChild!.type.name).toBe('softBreak')
})
it('attend la validation IME avant de retirer les retours initiaux', () => {
  const editor = create('<p></p>')
  Object.defineProperty(editor.view, 'composing', { configurable: true, value: true })
  editor.commands.setContent('<p></p><p><br data-soft-break>日本</p>')
  expect(editor.state.doc.firstChild!.textContent).toBe('')
  Object.defineProperty(editor.view, 'composing', { configurable: true, value: false })
  editor.view.dispatch(editor.state.tr.setMeta(documentWhitespaceKey, true))
  expect(children(editor).map((node) => node.textContent)).toEqual(['日本', ''])
})

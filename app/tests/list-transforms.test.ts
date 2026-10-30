import { afterEach, expect, it } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { planListSplit } from '../src/list-transforms'
const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((e) => e.destroy()))
function create(html: string) {
  const editor = createRichZone({ profile: 'document', label: 'List transforms', content: html })
  editors.push(editor)
  return editor
}
function at(editor: Editor, text: string, offset: number) {
  editor.state.doc.descendants((node, pos) => {
    if (node.isTextblock && node.textContent === text)
      editor.commands.setTextSelection(pos + 1 + offset)
  })
}
function press(editor: Editor, key: string) {
  return editor.view.someProp('handleKeyDown', (handler) =>
    handler(editor.view, new KeyboardEvent('keydown', { key })),
  )
}
it.each(['ul', 'ol'])(
  'split/join preserve rich content and provide symmetric keyboard deletion in %s',
  (tag) => {
    for (const key of ['Backspace', 'Delete']) {
      const editor = create(
        `<p>Avant</p><${tag}><li><p>A😀<strong>BC</strong><span data-footnote="Note"></span></p><p>Suite</p></li><li><p>Voisin</p></li></${tag}>`,
      )
      at(editor, 'A😀BC', 3)
      expect(press(editor, 'Enter')).toBe(true)
      const list = editor.state.doc.child(1)
      expect(list.childCount).toBe(3)
      expect(list.child(0).textContent).toBe('A😀')
      expect(list.child(1).childCount).toBe(2)
      expect(editor.state.selection.$from.parent.textContent).toBe('BC')
      expect(editor.state.selection.$from.parentOffset).toBe(0)
      if (key === 'Delete') at(editor, 'A😀', 3)
      expect(press(editor, key)).toBe(true)
      const joined = editor.state.doc.child(1)
      expect(joined.childCount).toBe(2)
      expect(joined.child(0).textContent).toBe('A😀BCSuite')
      expect(joined.child(0).child(0).lastChild?.type.name).toBe('footnote')
      expect(editor.state.selection.$from.parentOffset).toBe(3)
    }
  },
)
it('joins internal paragraphs with a soft break, not the neighboring list item', () => {
  const e = create('<ul><li><p>Un</p><p>Deux</p></li><li><p>Voisin</p></li></ul>')
  at(e, 'Deux', 0)
  press(e, 'Backspace')
  expect(e.state.doc.firstChild!.childCount).toBe(2)
  expect(e.state.doc.firstChild!.firstChild!.childCount).toBe(1)
  expect(e.state.doc.firstChild!.firstChild!.firstChild!.child(1).type.name).toBe('softBreak')
})
it('an empty item exits to the continuation; a note is never an empty item', () => {
  const e = create('<p>Avant</p><ul><li><p>Un</p></li><li><p></p></li></ul><p>Après</p>')
  e.state.doc.descendants((n, pos) => {
    if (n.type.name === 'listItem' && !n.textContent) e.commands.setTextSelection(pos + 2)
  })
  press(e, 'Enter')
  expect(e.state.doc.child(1).childCount).toBe(1)
  expect(e.state.selection.$from.depth).toBe(1)
  expect(e.state.selection.$from.parent.content.size).toBe(0)
  const note = create('<ul><li><p><span data-footnote="Ne pas perdre"></span></p><p></p></li></ul>')
  note.state.doc.descendants((n, pos, parent) => {
    if (parent?.type.name === 'listItem' && n.type.name === 'paragraph' && !n.content.size)
      note.commands.setTextSelection(pos + 1)
  })
  const plan = planListSplit(note)!
  note.view.dispatch(plan.transaction)
  expect(note.view.dom.querySelectorAll('.footnote-ref')).toHaveLength(1)
})
it('description terms remain separate, Enter splits the definition and focuses the new term', () => {
  const e = create(
    '<p>Avant</p><dl><div data-description-item><dt>Terme</dt><dd>Avant après</dd></div></dl>',
  )
  at(e, 'Avant après', 5)
  press(e, 'Enter')
  const list = e.state.doc.child(1)
  expect(list.childCount).toBe(2)
  expect(list.child(0).child(0).textContent).toBe('Terme')
  expect(list.child(0).child(1).textContent).toBe('Avant')
  expect(list.child(1).child(0).content.size).toBe(0)
  expect(list.child(1).child(1).textContent).toBe(' après')
  expect(e.state.selection.$from.parent.type.name).toBe('descriptionTerm')
  at(e, 'Avant', 5)
  const before = e.getJSON()
  press(e, 'Delete')
  expect(e.getJSON()).toEqual(before)
})
it('first item Backspace preserves the widget and returns to preceding prose', () => {
  const e = create('<p>Avant</p><ul><li><p>Premier</p></li></ul>')
  at(e, 'Premier', 0)
  press(e, 'Backspace')
  expect(e.state.selection.$from.parent.textContent).toBe('Avant')
  expect(e.state.doc.child(1).type.name).toBe('bulletList')
})

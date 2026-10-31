import { Editor } from '@tiptap/core'
import { afterEach, expect, it, vi } from 'vitest'
import { richZoneExtensions } from '../src/rich-zone'
import { locationDuration, revealEditorPosition } from '../src/editor-reveal'
const editors: Editor[] = []
afterEach(() => {
  editors.splice(0).forEach((e) => {
    if (!e.isDestroyed) e.destroy()
  })
  vi.useRealTimers()
  document.body.replaceChildren()
})
function create(content = '<p>Avant <strong>él</strong><em>éphant</em> !</p>') {
  const el = document.createElement('div')
  document.body.append(el)
  const e = new Editor({ element: el, extensions: richZoneExtensions('document'), content })
  editors.push(e)
  e.commands.setTextSelection(1)
  return e
}
const highlight = (e: Editor) =>
  [...e.view.dom.querySelectorAll('.sync-location')].map((n) => n.textContent).join('')
it('indique le mot Unicode entier à cheval sur les styles sans le sélectionner ni changer le contenu', () => {
  vi.useFakeTimers()
  const e = create(),
    before = e.getJSON()
  expect(revealEditorPosition(e, 9)).toBe(true)
  expect(highlight(e)).toBe('éléphant')
  expect(e.state.selection.empty).toBe(true)
  expect(e.state.selection.from).toBe(9)
  expect(e.getJSON()).toEqual(before)
  vi.advanceTimersByTime(locationDuration - 1)
  expect(highlight(e)).toBe('éléphant')
  vi.advanceTimersByTime(1)
  expect(highlight(e)).toBe('')
  expect(e.getJSON()).toEqual(before)
})
it('un nouveau repérage remplace le précédent et possède son propre délai', () => {
  vi.useFakeTimers()
  const e = create()
  revealEditorPosition(e, 2)
  vi.advanceTimersByTime(1000)
  revealEditorPosition(e, 9)
  vi.advanceTimersByTime(700)
  expect(highlight(e)).toBe('éléphant')
  vi.advanceTimersByTime(locationDuration - 700)
  expect(highlight(e)).toBe('')
})
it('la navigation et la saisie interrompent le repère immédiatement', () => {
  const e = create()
  revealEditorPosition(e, 9)
  e.commands.setTextSelection(2)
  expect(highlight(e)).toBe('')
  revealEditorPosition(e, 9)
  e.commands.insertContent('X')
  expect(highlight(e)).toBe('')
  expect(e.state.doc.firstChild!.textContent).toContain('X')
})
it('repère aussi un badge ou un paragraphe vide sans sélectionner leur contenu', () => {
  const e = create('<p>A<span data-footnote="Note"></span>B</p>')
  revealEditorPosition(e, 2)
  expect(e.view.dom.querySelector('.sync-location')).not.toBeNull()
  expect(e.state.selection.empty).toBe(true)
  revealEditorPosition(e, e.state.doc.content.size - 1)
  expect(e.view.dom.querySelector('p.sync-location')).not.toBeNull()
})
it('refuse une position invalide ou une navigation pendant la préédition et nettoie ses temporisateurs', () => {
  vi.useFakeTimers()
  const e = create()
  for (const pos of [-1, NaN, 100000, 1.5]) expect(revealEditorPosition(e, pos)).toBe(false)
  Object.defineProperty(e.view, 'composing', { configurable: true, value: true })
  expect(revealEditorPosition(e, 9)).toBe(false)
  Object.defineProperty(e.view, 'composing', { configurable: true, value: false })
  revealEditorPosition(e, 9)
  e.destroy()
  expect(() => vi.runAllTimers()).not.toThrow()
})

import { afterEach, expect, it } from 'vitest'
import { Fragment, Slice } from '@tiptap/pm/model'
import type { Editor } from '@tiptap/core'
import { createRichZone, type RichZoneProfile } from '../src/rich-zone'
import { protectedSelectionPlan, selectionKey } from '../src/selection-edits'
const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((e) => e.destroy()))
function create(html: string, profile: RichZoneProfile = 'document') {
  const element = document.createElement('div')
  document.body.append(element)
  const e = createRichZone({ profile, label: 'Protected selection', element, content: html })
  editors.push(e)
  return e
}
function select(e: Editor, a: string, b: string, from = 1, to = 2) {
  let first = 0,
    last = 0
  e.state.doc.descendants((n, pos) => {
    if (n.isTextblock) {
      if (n.textContent === a) first = pos + 1 + from
      if (n.textContent === b) last = pos + 1 + to
    }
  })
  e.commands.setTextSelection({ from: first, to: last })
}
const quote =
  '<p>Avant</p><blockquote><p>Corps</p><cite data-attribution>Source</cite></blockquote><p>Après</p>'
it.each(['Backspace', 'Delete', 'Enter'])(
  '%s preserves the quote body/attribution boundary and resumes in the first zone',
  (key) => {
    const e = create(quote)
    select(e, 'Corps', 'Source')
    expect(selectionKey(e, new KeyboardEvent('keydown', { key, cancelable: true }))).toBe(true)
    const widget = e.state.doc.child(1)
    expect(widget.type.name).toBe('quote')
    expect(widget.childCount).toBe(2)
    expect(widget.firstChild!.textContent).toBe('C')
    expect(widget.lastChild!.textContent).toBe('urce')
    expect(e.state.selection.$from.parent.type.name).toBe('paragraph')
    expect(e.state.doc.lastChild!.type.name).toBe('paragraph')
  },
)
it('replaces the selection once, retains rich marks, and removes only fully selected widgets', () => {
  const e = create(
    '<p>Début</p><blockquote><p>Corps</p><cite data-attribution>Source</cite></blockquote><p>Fin</p>',
  )
  select(e, 'Début', 'Source', 2, 2)
  const insert = new Slice(
    Fragment.from(e.schema.text('Ajout', [e.schema.marks.bold.create()])),
    0,
    0,
  )
  const plan = protectedSelectionPlan(e, insert)!
  e.view.dispatch(plan.transaction)
  expect(e.state.doc.child(0).textContent).toBe('DéAjout')
  expect(e.state.doc.child(0).lastChild!.marks[0].type.name).toBe('bold')
  expect(e.state.doc.child(1).type.name).toBe('quote')
  expect(e.state.doc.child(1).lastChild!.textContent).toBe('urce')
  const fully = create(
    '<p>Un</p><blockquote><p>Entier</p><cite data-attribution>Source 1</cite></blockquote><blockquote><p>Partiel</p><cite data-attribution>Source 2</cite></blockquote>',
  )
  select(fully, 'Un', 'Source 2', 1, 2)
  const replace = protectedSelectionPlan(fully)!
  fully.view.dispatch(replace.transaction)
  expect(fully.view.dom.querySelectorAll('blockquote')).toHaveLength(1)
  expect(fully.state.doc.child(1).lastChild!.textContent).toBe('urce 2')
})
it('preserves description term/definition and individual list item boundaries', () => {
  for (const html of [
    '<dl><div data-description-item><dt>Terme</dt><dd>Définition</dd></div></dl>',
    '<ul><li><p>Terme</p></li><li><p>Définition</p></li></ul>',
  ]) {
    const e = create(html)
    select(e, 'Terme', 'Définition', 2, 2)
    const plan = protectedSelectionPlan(e)!
    e.view.dispatch(plan.transaction)
    expect(e.state.doc.firstChild!.textContent).toBe('Tefinition')
    expect(e.state.doc.firstChild!.firstChild!.childCount).toBe(html.startsWith('<dl>') ? 2 : 1)
  }
})
it('uses the actual paste pipeline, flattens payload only at the insertion zone and leaves surrounding boundaries', () => {
  const e = create(quote)
  select(e, 'Corps', 'Source')
  e.view.pasteHTML(
    '<p><strong>Premier</strong></p><p><em>Second</em></p>',
    new Event('paste') as ClipboardEvent,
  )
  const widget = e.state.doc.child(1)
  expect(widget.firstChild!.textContent).toBe('CPremierSecond')
  expect(widget.firstChild!.child(2).type.name).toBe('softBreak')
  expect(widget.firstChild!.child(1).marks[0].type.name).toBe('bold')
  expect(widget.lastChild!.textContent).toBe('urce')
})
it.each(['inline-text', 'rich-text'] as const)(
  'paste respects %s profile without importing widgets or adding an unrelated root',
  (profile) => {
    const e = create('<p>Avant</p>', profile)
    e.commands.selectAll()
    e.view.pasteHTML(
      '<h1>Titre</h1><p><strong>Suite</strong></p>',
      new Event('paste') as ClipboardEvent,
    )
    expect(e.state.doc.content.content.every((n) => n.type.name === 'paragraph')).toBe(true)
    expect(e.getText()).toContain('Titre')
    expect(e.getText()).toContain('Suite')
    if (profile === 'inline-text') {
      expect(e.state.doc.childCount).toBe(1)
      expect(e.state.doc.firstChild!.textContent).toBe('Titre Suite')
      expect(e.state.doc.firstChild!.content.content.every((n) => n.isText)).toBe(true)
    }
  },
)
it('leaves ordinary prose joins, contextual shortcuts and IME preedit to their existing handlers', () => {
  const e = create('<p>Un</p><p>Deux</p>')
  select(e, 'Un', 'Deux', 1, 1)
  expect(protectedSelectionPlan(e)).toBeUndefined()
  const widget = create(quote)
  select(widget, 'Corps', 'Source')
  expect(selectionKey(widget, new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true }))).toBe(
    false,
  )
  expect(
    selectionKey(widget, new KeyboardEvent('keydown', { key: 'Backspace', isComposing: true })),
  ).toBe(false)
})

it('bulk native text keeps explicit inline breaks instead of hidden newline characters', () => {
  const e = create(quote)
  select(e, 'Corps', 'Source')
  e.view.dom.dispatchEvent(
    new InputEvent('beforeinput', {
      bubbles: true,
      cancelable: true,
      inputType: 'insertText',
      data: 'Première\nSeconde',
    }),
  )
  const paragraph = e.state.doc.child(1).firstChild!
  expect(paragraph.textContent).toBe('CPremièreSeconde')
  expect(paragraph.content.content.some((node) => node.type.name === 'softBreak')).toBe(true)
  expect(paragraph.content.content.every((node) => !node.text?.includes('\n'))).toBe(true)
})

it('never destructively cuts without clipboard access and never prepares IME in a read-only zone', () => {
  const e = create(quote)
  select(e, 'Corps', 'Source')
  const before = e.getJSON()
  e.view.dom.dispatchEvent(new Event('cut', { bubbles: true, cancelable: true }))
  expect(e.getJSON()).toEqual(before)
  e.setEditable(false)
  e.view.dom.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true, data: '' }))
  expect(e.getJSON()).toEqual(before)
})

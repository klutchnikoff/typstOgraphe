import { Editor } from '@tiptap/core'
import { afterEach, expect, it } from 'vitest'
import { richZoneExtensions, type RichZoneProfile } from '../src/rich-zone'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))

function create(profile: RichZoneProfile, quote = false) {
  const html = '<p>Avant</p><p>Après</p>'
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions(profile),
    content: quote ? `<blockquote>${html}</blockquote>` : html,
  })
  editors.push(editor)
  editor.commands.setTextSelection(1) // Run the shared initial whitespace repair.
  const paragraphs: number[] = []
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name === 'paragraph' && node.content.size) paragraphs.push(pos + 1)
  })
  return { editor, first: paragraphs[0], second: paragraphs[1] }
}
function press(editor: Editor, key: string, modifiers: KeyboardEventInit = {}) {
  return !!editor.view.someProp('handleKeyDown', (handler) =>
    handler(editor.view, new KeyboardEvent('keydown', { key, ...modifiers })),
  )
}

it.each([
  ['document', false],
  ['document', true],
  ['rich-text', false],
] as const)('franchit le marqueur de paragraphe dans %s (citation : %s)', (profile, quote) => {
  const { editor, first, second } = create(profile, quote)
  const before = editor.getJSON()
  editor.commands.setTextSelection(first + 5)
  expect(press(editor, 'ArrowRight')).toBe(true)
  expect(editor.state.selection.from).toBe(second)
  expect(press(editor, 'ArrowLeft')).toBe(true)
  expect(editor.state.selection.from).toBe(first + 5)

  // Shift preserves the anchor in either direction, including a selection
  // that already covers text. The decorative symbol is never document content.
  editor.commands.setTextSelection({ from: first, to: first + 5 })
  expect(press(editor, 'ArrowRight', { shiftKey: true })).toBe(true)
  expect(editor.state.selection.anchor).toBe(first)
  expect(editor.state.selection.head).toBe(second)
  editor.commands.setTextSelection({ from: second + 5, to: second })
  expect(press(editor, 'ArrowLeft', { shiftKey: true })).toBe(true)
  expect(editor.state.selection.anchor).toBe(second + 5)
  expect(editor.state.selection.head).toBe(first + 5)
  expect(editor.getJSON()).toEqual(before)
})

it('laisse au navigateur les déplacements internes, par mot, la réduction de sélection et l’IME', () => {
  const { editor, first } = create('document')
  editor.commands.setTextSelection(first + 2)
  expect(press(editor, 'ArrowRight')).toBe(false)
  editor.commands.setTextSelection({ from: first, to: first + 5 })
  expect(press(editor, 'ArrowLeft')).toBe(false)
  editor.commands.setTextSelection(first + 5)
  for (const modifiers of [
    { altKey: true },
    { ctrlKey: true },
    { metaKey: true },
    { isComposing: true },
  ])
    expect(press(editor, 'ArrowRight', modifiers)).toBe(false)
  Object.defineProperty(editor.view, 'composing', { configurable: true, value: true })
  expect(press(editor, 'ArrowRight')).toBe(false)
  expect(editor.state.selection.from).toBe(first + 5)
})

it.each(['document', 'rich-text'] as const)(
  'utilise la sélection DOM courante si ProseMirror est encore en retard (%s)',
  (profile) => {
    const { editor, first, second } = create(profile)
    document.body.append(editor.view.dom)
    editor.view.focus()
    const [before, after] = Array.from(editor.view.dom.querySelectorAll('p')).map(
      (p) => p.firstChild!,
    )
    const dom = document.getSelection()!
    // selectionchange is asynchronous. The native caret has already crossed
    // the marker, while PM still holds the previous end-of-paragraph selection.
    editor.commands.setTextSelection({ from: first, to: first + 5 })
    dom.setBaseAndExtent(before, 0, after, 2)
    expect(editor.state.selection.head).toBe(first + 5)
    expect(press(editor, 'ArrowRight', { shiftKey: true })).toBe(false)
    expect(dom.focusNode).toBe(after)
    expect(dom.focusOffset).toBe(2)
    // Conversely, a native caret at the boundary must cross even when PM's
    // previous selection is still inside the paragraph.
    editor.commands.setTextSelection({ from: first, to: first + 3 })
    dom.setBaseAndExtent(before, 0, before, 5)
    expect(press(editor, 'ArrowRight', { shiftKey: true })).toBe(true)
    expect(editor.state.selection.anchor).toBe(first)
    expect(editor.state.selection.head).toBe(second)
    editor.commands.setTextSelection({ from: second + 5, to: second + 2 })
    dom.setBaseAndExtent(after, 5, after, 0)
    expect(press(editor, 'ArrowLeft', { shiftKey: true })).toBe(true)
    expect(editor.state.selection.anchor).toBe(second + 5)
    expect(editor.state.selection.head).toBe(first + 5)
    editor.view.dom.remove()
  },
)

it.each(['document', 'rich-text'] as const)(
  'un rafraîchissement sans édition préserve les marques ¶ et la sélection native (%s)',
  (profile) => {
    const { editor, first } = create(profile)
    document.body.append(editor.view.dom)
    editor.view.focus()
    editor.commands.setTextSelection({ from: first, to: first + 1 })
    const paragraph = editor.view.dom.querySelector('p')!
    const text = paragraph.firstChild!
    const marker = editor.view.dom.querySelector('.paragraph-marker')!
    const dom = document.getSelection()!
    dom.setBaseAndExtent(text, 0, text, 4)
    const before = editor.getJSON()
    // Native selectionchange is still queued. A focus/status/decoration refresh
    // must not replace a purely decorative node and reset the browser's range.
    editor.view.dispatch(editor.state.tr.setMeta('status-refresh', true))
    expect(dom.focusOffset).toBe(4)
    expect(editor.view.dom.querySelector('.paragraph-marker')).toBe(marker)
    expect(dom.anchorNode).toBe(text)
    expect(dom.anchorOffset).toBe(0)
    expect(dom.focusNode).toBe(text)
    expect(editor.getJSON()).toEqual(before)
    editor.view.dom.remove()
  },
)

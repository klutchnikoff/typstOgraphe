import { afterEach, expect, it, vi } from 'vitest'
import { Editor, Node } from '@tiptap/core'
import { NodeSelection } from '@tiptap/pm/state'
import { baseExtensions } from '../src/extensions'
import { inlineObjectKey, inlineObjectTarget } from '../src/inline-objects'
import { correctionAtCursor, proofingKey } from '../src/proofing'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
function create(kind = 'footnote') {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: [
      ...baseExtensions(() => ''),
      Node.create({
        name: 'futureReference',
        group: 'inline',
        inline: true,
        atom: true,
        renderHTML: () => ['span', 'Reference'],
      }),
    ],
    content: {
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Fote' }, { type: kind }] }],
    },
  })
  editors.push(editor)
  editor.commands.setTextSelection(1)
  return editor
}
it('la cible est l’objet sélectionné ou voisin immédiat, jamais le mot ou une sélection de texte', () => {
  const editor = create()
  for (const pos of [5, 6]) {
    editor.commands.setTextSelection(pos)
    expect(inlineObjectTarget(editor)?.pos).toBe(5)
  }
  editor.commands.setTextSelection(3)
  expect(inlineObjectTarget(editor)).toBeUndefined()
  editor.commands.setTextSelection({ from: 1, to: 5 })
  expect(inlineObjectTarget(editor)).toBeUndefined()
  editor.commands.setNodeSelection(5)
  expect(inlineObjectTarget(editor)?.node.type.name).toBe('footnote')
  editor.commands.setContent('<p>A<br data-soft-break>B</p>')
  editor.commands.setTextSelection(2)
  expect(inlineObjectTarget(editor)).toBeUndefined()
})
it.each(['footnote', 'futureReference'])(
  '%s passe par la confirmation partagée, quel que soit le sens de suppression',
  (kind) => {
    const editor = create(kind),
      open = vi.fn(),
      original = editor.getJSON()
    for (const [pos, key] of [
      [6, 'Backspace'],
      [5, 'Delete'],
    ] as const) {
      editor.commands.setTextSelection(pos)
      const event = new KeyboardEvent('keydown', { key, cancelable: true })
      expect(inlineObjectKey(editor, event, open)).toBe(true)
      expect(event.defaultPrevented).toBe(true)
      expect(open).toHaveBeenLastCalledWith(5, kind, true)
      expect(editor.getJSON()).toEqual(original)
    }
    editor.commands.setNodeSelection(5)
    expect(inlineObjectKey(editor, new KeyboardEvent('keydown', { key: 'Backspace' }), open)).toBe(
      true,
    )
    expect(editor.state.selection).toBeInstanceOf(NodeSelection)
    expect(editor.getJSON()).toEqual(original)
    editor.commands.setTextSelection(5)
    expect(inlineObjectKey(editor, new KeyboardEvent('keydown', { key: 'Backspace' }), open)).toBe(
      false,
    )
    editor.commands.setTextSelection(6)
    expect(
      inlineObjectKey(
        editor,
        new KeyboardEvent('keydown', { key: 'Backspace', isComposing: true }),
        open,
      ),
    ).toBe(false)
  },
)
it('une sélection de note ne devient pas une correction du mot qui la précède', () => {
  const editor = create()
  editor.view.dispatch(
    editor.state.tr.setMeta(proofingKey, { issues: [{ from: 1, to: 5, word: 'Fote' }] }),
  )
  editor.commands.setNodeSelection(5)
  expect(correctionAtCursor(editor)).toBeUndefined()
  editor.commands.setTextSelection(3)
  expect(correctionAtCursor(editor)?.word).toBe('Fote')
})

it.each(['footnote', 'futureReference'])(
  'les flèches traversent %s avec un curseur de texte, sans sélectionner un objet à remplacer',
  (kind) => {
    const editor = create(kind),
      open = vi.fn(),
      original = editor.getJSON()
    for (const [from, key, to] of [
      [6, 'ArrowLeft', 5],
      [5, 'ArrowRight', 6],
    ] as const) {
      editor.commands.setTextSelection(from)
      const event = new KeyboardEvent('keydown', { key, cancelable: true })
      expect(inlineObjectKey(editor, event, open)).toBe(true)
      expect(event.defaultPrevented).toBe(true)
      expect(editor.state.selection.empty).toBe(true)
      expect(editor.state.selection.from).toBe(to)
      expect(editor.getJSON()).toEqual(original)
    }
    editor.commands.setNodeSelection(5)
    expect(inlineObjectKey(editor, new KeyboardEvent('keydown', { key: 'ArrowLeft' }), open)).toBe(
      true,
    )
    expect(editor.state.selection.empty).toBe(true)
    expect(editor.state.selection.from).toBe(5)
    for (const modifier of ['shiftKey', 'altKey', 'ctrlKey', 'metaKey', 'isComposing']) {
      editor.commands.setTextSelection(6)
      expect(
        inlineObjectKey(
          editor,
          new KeyboardEvent('keydown', { key: 'ArrowLeft', [modifier]: true }),
          open,
        ),
      ).toBe(false)
    }
    expect(open).not.toHaveBeenCalled()
  },
)

it.each(['footnote', 'futureReference'])(
  'la sortie de %s rejoint le texte voisin par graphème, sans arrêt DOM supplémentaire',
  (kind) => {
    const editor = create(kind),
      open = vi.fn(),
      before = 'e\u0301👨‍👩‍👧‍👦',
      after = '👩🏽‍💻e\u0301'
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [{ type: 'text', text: before }, { type: kind }, { type: 'text', text: after }],
        },
      ],
    })
    const note = 1 + before.length,
      original = editor.getJSON()
    for (const [from, key, to] of [
      [note, 'ArrowLeft', 3],
      [note + 1, 'ArrowRight', note + 1 + '👩🏽‍💻'.length],
    ] as const) {
      editor.commands.setTextSelection(from)
      const event = new KeyboardEvent('keydown', { key, cancelable: true })
      expect(inlineObjectKey(editor, event, open)).toBe(true)
      expect(editor.state.selection.from).toBe(to)
      expect(event.defaultPrevented).toBe(true)
      expect(editor.getJSON()).toEqual(original)
    }
    for (const modifier of ['shiftKey', 'altKey', 'ctrlKey', 'metaKey', 'isComposing']) {
      editor.commands.setTextSelection(note + 1)
      expect(
        inlineObjectKey(
          editor,
          new KeyboardEvent('keydown', { key: 'ArrowRight', [modifier]: true }),
          open,
        ),
      ).toBe(false)
    }
    expect(open).not.toHaveBeenCalled()
  },
)

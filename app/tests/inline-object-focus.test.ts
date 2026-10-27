import { afterEach, expect, it } from 'vitest'
import { Editor, Node } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { activeInlineObject } from '../src/inline-object-focus'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
function create(kind: string) {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: [
      ...richZoneExtensions('document'),
      Node.create({
        name: 'futureCitation',
        group: 'inline',
        inline: true,
        atom: true,
        renderHTML: () => ['span', 'Auteur, 2026'],
      }),
    ],
    content: {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'Texte' },
            { type: kind },
            { type: kind },
            { type: 'text', text: ' suite' },
          ],
        },
      ],
    },
  })
  editors.push(editor)
  editor.commands.setTextSelection(1)
  return editor
}
it.each(['footnote', 'futureCitation'])(
  'l’activation de %s suit uniquement le point d’insertion à droite et désigne l’objet précédent entre deux badges',
  (kind) => {
    const editor = create(kind),
      original = editor.getJSON()
    for (const [cursor, target] of [
      [6, undefined],
      [7, 6],
      [8, 7],
      [9, undefined],
    ] as const) {
      editor.commands.setTextSelection(cursor)
      expect(activeInlineObject(editor.state)?.pos).toBe(target)
      const active = editor.view.dom.querySelectorAll('.inline-object-active')
      expect(active).toHaveLength(target === undefined ? 0 : 1)
      if (target !== undefined) expect(active[0]).toBe(editor.view.nodeDOM(target))
    }
    editor.commands.setTextSelection({ from: 5, to: 7 })
    expect(activeInlineObject(editor.state)).toBeUndefined()
    expect(editor.view.dom.querySelector('.inline-object-active')).toBeNull()
    editor.commands.setNodeSelection(6)
    expect(activeInlineObject(editor.state)).toBeUndefined()
    expect(editor.view.dom.querySelector('.inline-object-active')).toBeNull()
    expect(editor.getJSON()).toEqual(original)
    expect(editor.getHTML()).not.toContain('inline-object-active')
  },
)

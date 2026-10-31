import { Editor, Node } from '@tiptap/core'
import { afterEach, expect, it } from 'vitest'
import { richZoneExtensions } from '../src/rich-zone'
import { inlineObjectView } from '../src/inline-objects'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
function create(kind: string, prefix: object[]) {
  const editor = new Editor({
    extensions: [
      ...richZoneExtensions('document'),
      Node.create({
        name: 'futureBadge',
        group: 'inline',
        inline: true,
        atom: true,
        renderHTML: () => ['span', { 'data-future-badge': '' }],
        addNodeView:
          () =>
          ({ editor, getPos }) => ({
            dom: inlineObjectView({
              editor,
              getPos,
              kind: 'futureBadge',
              label: 'Future',
              title: 'Future',
              open: () => {},
            }).dom,
          }),
      }),
    ],
    content: {
      type: 'doc',
      content: [{ type: 'paragraph', content: [...prefix, { type: kind }] }],
    },
  })
  editors.push(editor)
  return editor
}
const text = (text: string) => ({ type: 'text', text })
for (const kind of ['footnote', 'citation', 'futureBadge']) {
  it.each([
    ['début de paragraphe', [], true],
    ['mot', [text('Voir')], false],
    ['espace', [text('Voir ')], true],
    ['espace insécable', [text('Voir\u00a0')], true],
    ['espace fine insécable', [text('Voir\u202f')], true],
    [
      'espace dans une autre mise en forme',
      [text('Voir'), { ...text(' '), marks: [{ type: 'italic' }] }],
      true,
    ],
    ['retour simple', [text('Voir'), { type: 'softBreak' }], true],
    ['retour forcé', [text('Voir'), { type: 'hardBreak' }], true],
  ] as const)(
    '%s : l’espacement de ' + kind + ' est seulement une décoration locale',
    (_label, prefix, flush) => {
      const editor = create(kind, [...prefix])
      expect(
        editor.view.dom.querySelector('.inline-object')?.classList.contains('inline-object-flush'),
      ).toBe(flush)
      expect(editor.getHTML()).not.toContain('inline-object-flush')
      expect(JSON.stringify(editor.getJSON())).not.toContain('inline-object-flush')
      expect(editor.getText()).not.toContain('\u2060')
    },
  )
}
it('l’espace est reconnue après insertion et suppression sans ajouter de caractère au badge', () => {
  const editor = create('footnote', [text('Voir')])
  const badge = editor.view.dom.querySelector('.inline-object')!
  expect(badge.classList.contains('inline-object-flush')).toBe(false)
  editor.commands.insertContentAt(5, ' ')
  expect(editor.view.dom.querySelector('.inline-object')).toBe(badge)
  expect(badge.classList.contains('inline-object-flush')).toBe(true)
  editor.commands.deleteRange({ from: 5, to: 6 })
  expect(badge.classList.contains('inline-object-flush')).toBe(false)
  expect(editor.state.doc.firstChild!.textContent).toBe('Voir')
})

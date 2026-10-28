import { afterEach, describe, expect, it } from 'vitest'
import { Editor, type JSONContent } from '@tiptap/core'
import { createTable } from '@tiptap/extension-table'
import { richZoneExtensions } from '../src/rich-zone'
import { insertBlock, listContent, canInsertBlock } from '../src/editing'

const editors: Editor[] = []
afterEach(() => {
  for (const editor of editors.splice(0)) editor.destroy()
})
function editor(content: string | JSONContent, pos = 1) {
  const value = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions('document'),
    content,
  })
  editors.push(value)
  value.commands.setTextSelection(pos)
  return value
}
const widgets = [
  'tableFigure',
  'figure',
  'quote',
  'bulletList',
  'orderedList',
  'descriptionList',
  'postscript',
  'attachment',
] as const
function widget(e: Editor, kind: (typeof widgets)[number]): JSONContent {
  if (kind === 'tableFigure')
    return {
      type: kind,
      content: [createTable(e.schema, 3, 2, true).toJSON(), { type: 'tableCaption' }],
    }
  if (kind === 'figure') return { type: kind, attrs: { src: 'images/test.svg', alt: 'Image' } }
  if (kind === 'quote')
    return { type: kind, content: [{ type: 'paragraph' }, { type: 'quoteAttribution' }] }
  if (['bulletList', 'orderedList', 'descriptionList'].includes(kind))
    return listContent(kind as 'bulletList')
  return { type: kind }
}
const lines = (e: Editor) =>
  e.state.doc.content.content.map((node) => [node.type.name, node.textContent])

describe.each(widgets)('insertion de %s', (kind) => {
  it.each([
    [
      'début de ligne',
      1,
      [
        [kind, ''],
        ['paragraph', ''],
        ['paragraph', 'AvantAprès'],
        ['paragraph', ''],
      ],
    ],
    [
      'milieu de ligne',
      6,
      [
        ['paragraph', 'Avant'],
        [kind, ''],
        ['paragraph', ''],
        ['paragraph', 'Après'],
        ['paragraph', ''],
      ],
    ],
    [
      'fin de ligne',
      11,
      [
        ['paragraph', 'AvantAprès'],
        [kind, ''],
        ['paragraph', ''],
      ],
    ],
  ] as const)('%s : aucun blanc avant, une ligne de reprise après', (_, pos, expected) => {
    const e = editor('<p>AvantAprès</p>', pos)
    expect(insertBlock(e, widget(e, kind))).toBe(true)
    expect(lines(e)).toEqual(expected)
    const selected = e.state.selection.$from
    expect(
      Array.from({ length: selected.depth + 1 }, (_, depth) => selected.node(depth).type.name),
    ).toContain(kind)
  })
  it('remplace la ligne vide courante et réutilise la ligne vide suivante', () => {
    const e = editor('<p></p><p></p><p>Suite</p>')
    expect(insertBlock(e, widget(e, kind))).toBe(true)
    expect(lines(e)).toEqual([
      [kind, ''],
      ['paragraph', ''],
      ['paragraph', 'Suite'],
      ['paragraph', ''],
    ])
  })
})
it('un retour simple ne laisse pas de ligne vide avant le tableau', () => {
  const e = editor(
    {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'Avant' },
            { type: 'softBreak' },
            { type: 'text', text: 'Après' },
          ],
        },
      ],
    },
    7,
  )
  insertBlock(e, widget(e, 'tableFigure'))
  expect(lines(e)).toEqual([
    ['paragraph', 'Avant'],
    ['tableFigure', ''],
    ['paragraph', ''],
    ['paragraph', 'Après'],
    ['paragraph', ''],
  ])
  expect(e.state.doc.firstChild!.lastChild!.type.name).toBe('text')
})
it('le texte non sélectionné conserve sa mise en forme de part et d’autre', () => {
  const e = editor('<p><strong>Avant</strong>choisi<em>Après</em></p>')
  e.commands.setTextSelection({ from: 6, to: 12 })
  insertBlock(e, widget(e, 'quote'))
  expect(lines(e)).toEqual([
    ['paragraph', 'Avant'],
    ['quote', ''],
    ['paragraph', ''],
    ['paragraph', 'Après'],
    ['paragraph', ''],
  ])
  expect(e.state.doc.firstChild!.firstChild!.marks[0].type.name).toBe('bold')
  expect(e.state.doc.child(e.state.doc.childCount - 2).firstChild!.marks[0].type.name).toBe(
    'italic',
  )
})
it('ne sort pas implicitement d’une cellule pour insérer un autre widget', () => {
  const e = editor('<p></p>')
  insertBlock(e, widget(e, 'tableFigure'))
  const before = e.getJSON()
  expect(canInsertBlock(e)).toBe(false)
  expect(insertBlock(e, widget(e, 'quote'))).toBe(false)
  expect(e.getJSON()).toEqual(before)
})

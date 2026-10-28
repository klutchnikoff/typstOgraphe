import { afterEach, expect, it } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { bindEditorTarget } from '../src/editor-targets'
import { replacementPlan, replaceText } from '../src/text-edits'
import { findMatches } from '../src/proofing'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
async function create(content: Parameters<typeof createRichZone>[0]['content']) {
  const editor = createRichZone({ profile: 'document', label: 'Corrections', content })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  bindEditorTarget(editor, { kind: 'body' })
  return editor
}
it('plans descending UTF-16 replacements across marks and cells without flattening zones', async () => {
  const editor = await create({
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: '😀 mot ', marks: [{ type: 'bold' }] },
          {
            type: 'text',
            text: 'mot',
            marks: [{ type: 'italic' }, { type: 'link', attrs: { href: 'https://example.org' } }],
          },
        ],
      },
      {
        type: 'tableFigure',
        content: [
          {
            type: 'table',
            content: [
              {
                type: 'tableRow',
                content: [
                  {
                    type: 'tableCell',
                    content: [{ type: 'paragraph', content: [{ type: 'text', text: 'mot' }] }],
                  },
                ],
              },
            ],
          },
          { type: 'tableCaption' },
        ],
      },
    ],
  })
  const hits = findMatches(editor.state.doc, 'mot', true)
  const plan = replacementPlan(
    editor,
    hits.map((hit) => ({ ...hit, text: '<suite>' })),
  )
  expect(plan.commands).toMatchObject([
    {
      id: 'text.insert',
      params: {
        target: { zone: { kind: 'body', path: [1, 0, 0, 0, 0] }, from: 0, to: 3 },
        text: '<suite>',
        marks: [],
      },
    },
    {
      id: 'text.insert',
      params: {
        target: { zone: { path: [0] }, from: 7, to: 10 },
        marks: [{ type: 'link', url: 'https://example.org' }, { type: 'italic' }],
      },
    },
    {
      id: 'text.insert',
      params: { target: { zone: { path: [0] }, from: 3, to: 6 }, marks: [{ type: 'bold' }] },
    },
  ])
  expect(editor.state.doc.textContent).toBe('😀 mot motmot')
  expect(plan.transaction.doc.textContent).toBe('😀 <suite> <suite><suite>')
})
it('protects badges, returns, zone boundaries, stale text and surrogate pairs', async () => {
  const editor = await create({
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: '😀mot' },
          { type: 'footnote', attrs: { text: 'mot secret' } },
          { type: 'text', text: 'suite' },
          { type: 'softBreak' },
          { type: 'text', text: 'fin' },
        ],
      },
      { type: 'paragraph', content: [{ type: 'text', text: 'mot' }] },
    ],
  })
  expect(findMatches(editor.state.doc, 'mot secret', true)).toEqual([])
  expect(findMatches(editor.state.doc, '\ufffc', true)).toEqual([])
  expect(findMatches(editor.state.doc, 'suite\nfin', true)).toEqual([])
  const hit = findMatches(editor.state.doc, 'mot', true)[0]
  for (const range of [
    { ...hit, word: 'ancien' },
    { from: 1, to: 2, word: '\ud83d' },
    { from: 6, to: 7, word: '\ufffc' },
    { from: hit.from, to: 30, word: 'zones' },
  ])
    expect(() => replacementPlan(editor, [{ ...range, text: 'texte' }])).toThrow()
  expect(() => replacementPlan(editor, [{ ...hit, text: 'a\nb' }])).toThrow()
  expect(() =>
    replacementPlan(editor, [
      { ...hit, text: 'a' },
      { ...hit, text: 'b' },
    ]),
  ).toThrow()
})
it('uses record roots and keeps detached draft replacements local and literal', async () => {
  const editor = await create('<p><strong>mot</strong></p>')
  bindEditorTarget(editor, {
    kind: 'record',
    collection: 'localizations',
    id: 'fr',
    field: 'abstract',
  })
  const hit = findMatches(editor.state.doc, 'mot', true)[0]
  expect(replacementPlan(editor, [{ ...hit, text: 'corrigé' }]).commands[0].params.target).toEqual({
    zone: { kind: 'record', collection: 'localizations', id: 'fr', field: 'abstract', path: [0] },
    from: 0,
    to: 3,
  })
  await replaceText(editor, [{ ...hit, text: '*texte* <littéral>' }], { focus: false })
  expect(editor.state.doc.firstChild?.firstChild?.marks[0].type.name).toBe('bold')
  expect(editor.state.doc.textContent).toBe('*texte* <littéral>')
})

it('maps the planned caret through empty paragraph repair before installing the owner reply', async () => {
  const editor = await create('<p>mot</p><p>suite</p>')
  editor.commands.setTextSelection({ from: 1, to: 4 })
  const plan = replacementPlan(editor, [{ from: 1, to: 4, word: 'mot', text: '' }])
  expect(plan.transaction.doc.firstChild?.textContent).toBe('suite')
  expect(plan.transaction.selection.from).toBe(1)
  expect(plan.transaction.doc.lastChild?.content.size).toBe(0)
})

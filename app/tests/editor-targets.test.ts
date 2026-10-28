import { afterEach, expect, it } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { bindEditorTarget, textTargets } from '../src/editor-targets'
import { applyFormatting, formattingBatch } from '../src/formatting'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
async function create(content: unknown) {
  const editor = createRichZone({
    profile: 'document',
    label: 'Targets',
    content: content as Parameters<typeof createRichZone>[0]['content'],
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  bindEditorTarget(editor, { kind: 'body' })
  return editor
}
it('maps Unicode, marks and inline objects without flattening their labels', async () => {
  const editor = await create({
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'A😀', marks: [{ type: 'bold' }] },
          { type: 'footnote', attrs: { text: 'A very long note' } },
          { type: 'text', text: 'é日本' },
        ],
      },
      { type: 'paragraph', content: [{ type: 'text', text: 'Suite' }] },
    ],
  })
  editor.commands.setTextSelection({ from: 2, to: 12 })
  expect(textTargets(editor)).toEqual([
    { zone: { kind: 'body', path: [0] }, from: 1, to: 7 },
    { zone: { kind: 'body', path: [1] }, from: 0, to: 2 },
  ])
  expect(
    formattingBatch(editor, 'text.bold', editor.state.selection).map((c) => c.params.value),
  ).toEqual(['on', 'on'])
})
it('addresses terms, definitions, cells, captions and attributions independently', async () => {
  const editor = await create({
    type: 'doc',
    content: [
      {
        type: 'descriptionList',
        content: [
          {
            type: 'descriptionItem',
            content: [
              { type: 'descriptionTerm', content: [{ type: 'text', text: 'Terme' }] },
              { type: 'descriptionDefinition', content: [{ type: 'text', text: 'Définition' }] },
            ],
          },
        ],
      },
      {
        type: 'quote',
        content: [
          { type: 'paragraph', content: [{ type: 'text', text: 'Texte' }] },
          { type: 'quoteAttribution', content: [{ type: 'text', text: 'Source' }] },
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
                    content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Cellule' }] }],
                  },
                ],
              },
            ],
          },
          { type: 'tableCaption', content: [{ type: 'text', text: 'Légende' }] },
        ],
      },
    ],
  })
  editor.commands.selectAll()
  expect(textTargets(editor).map((t) => t.zone.path)).toEqual([
    [0, 0, 0],
    [0, 0, 1],
    [1, 0],
    [1, 1],
    [2, 0, 0, 0, 0],
    [2, 1],
  ])
})
it('binds record identity rather than its visual order and leaves an empty selection to typing', async () => {
  const editor = await create('<p>Résumé 😀</p><p>Suite</p>')
  bindEditorTarget(editor, {
    kind: 'record',
    collection: 'localizations',
    id: 'fr',
    field: 'abstract',
  })
  editor.commands.setTextSelection({ from: 1, to: 10 })
  expect(formattingBatch(editor, 'text.italic', editor.state.selection)).toEqual([
    {
      id: 'text.italic',
      params: {
        value: 'on',
        target: {
          zone: {
            kind: 'record',
            collection: 'localizations',
            id: 'fr',
            field: 'abstract',
            path: [0],
          },
          from: 0,
          to: 9,
        },
      },
    },
  ])
  editor.commands.setTextSelection(10)
  const before = editor.getJSON()
  await applyFormatting(editor, 'text.bold', { focus: false })
  expect(editor.getJSON()).toEqual(before)
  expect(editor.isActive('bold')).toBe(true)
  expect(textTargets(editor)).toEqual([])
})
it('translates headings explicitly and converts an active heading back to a paragraph', async () => {
  const editor = await create('<h2>Titre</h2><p>Suite</p>')
  editor.commands.setTextSelection(2)
  expect(formattingBatch(editor, 'block.heading_2', editor.state.selection)).toEqual([
    { id: 'block.paragraph', params: { target: { root: { kind: 'body' }, path: [0] } } },
  ])
  editor.commands.setTextSelection({ from: 2, to: 10 })
  expect(formattingBatch(editor, 'block.heading_1', editor.state.selection)).toHaveLength(2)
})

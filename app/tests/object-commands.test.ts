import { afterEach, expect, it } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { bindEditorTarget } from '../src/editor-targets'
import { inlineTarget, widgetBatch } from '../src/object-commands'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
async function create() {
  const editor = createRichZone({
    profile: 'document',
    label: 'Widget targets',
    content: '<p>A😀é</p><p>Conserver</p>',
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  bindEditorTarget(editor, { kind: 'body' })
  return editor
}
it('splices only changed blocks and never serializes binding-private mark attributes', async () => {
  const editor = await create()
  const quote = editor.schema.nodes.quote.create(null, [
    editor.schema.nodes.paragraph.create(null, editor.schema.text('Citation')),
    editor.schema.nodes.quoteAttribution.create(),
  ])
  const first = editor.state.doc.firstChild!
  const transaction = editor.state.tr.replaceWith(0, first.nodeSize, quote)
  expect(widgetBatch(editor, transaction)).toEqual([
    {
      id: 'block.splice',
      params: {
        parent: { root: { kind: 'body' }, path: [] },
        index: 0,
        delete_count: 1,
        nodes: [
          {
            type: 'quote',
            content: [
              { type: 'paragraph', content: [{ type: 'text', text: 'Citation' }] },
              { type: 'quoteAttribution' },
            ],
          },
        ],
      },
    },
  ])
  expect(editor.state.doc.child(1).textContent).toBe('Conserver')
  expect(widgetBatch(editor, editor.state.tr)).toEqual([])
})
it('addresses inline ranges in UTF-16 and rejects cross-zone insertion before publication', async () => {
  const editor = await create()
  expect(inlineTarget(editor, 4, 5)).toEqual({ zone: { kind: 'body', path: [0] }, from: 3, to: 4 })
  expect(() => inlineTarget(editor, 4, 8)).toThrow('Invalid inline range')
  const atom = editor.schema.nodes.footnote.create(
    { text: 'Note', __marks: [{ type: 'bold' }] },
    null,
  )
  const paragraph = editor.schema.nodes.paragraph.create(null, atom)
  const command = widgetBatch(
    editor,
    editor.state.tr.replaceWith(0, editor.state.doc.firstChild!.nodeSize, paragraph),
  )[0]
  expect(JSON.stringify(command)).not.toContain('__marks')
  expect(JSON.stringify(command)).toContain('"marks":[{"type":"bold"}]')
})

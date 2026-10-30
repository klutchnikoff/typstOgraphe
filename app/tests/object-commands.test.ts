import { afterEach, expect, it } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { bindEditorTarget } from '../src/editor-targets'
import { applyObjectEdit, inlineTarget, widgetBatch } from '../src/object-commands'

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

it('a widget conversion keeps a directed text range when its content is unchanged, without taking form focus', async () => {
  const editor = await create()
  editor.commands.setContent(
    '<ol><li><p>Premier</p></li><li><p>Deuxième 😀</p></li></ol><p>Suite</p>',
  )
  let pos = 0
  editor.state.doc.descendants((node, at) => {
    if (node.type.name === 'paragraph' && node.textContent === 'Deuxième 😀') pos = at + 1
  })
  editor.commands.setTextSelection({ from: pos + 8, to: pos + 2 })
  const selection = editor.state.selection.toJSON()
  const list = editor.state.doc.firstChild!
  const field = document.createElement('input')
  document.body.append(field)
  field.focus()
  try {
    const replacement = editor.schema.nodes.bulletList.create(null, list.content)
    await applyObjectEdit(
      editor,
      editor.state.tr.replaceWith(0, list.nodeSize, replacement),
      undefined,
      { focus: false },
    )
    expect(editor.state.doc.firstChild!.type.name).toBe('bulletList')
    expect(editor.state.selection.toJSON()).toEqual(selection)
    expect(document.activeElement).toBe(field)
  } finally {
    field.remove()
  }
})
it('replacement with different content enters its first zone rather than the following paragraph', async () => {
  const editor = await create()
  editor.commands.setContent('<ol><li><p>Ancien contenu</p></li></ol><p>Après</p>')
  editor.commands.setTextSelection(6)
  const node = editor.state.doc.firstChild!
  const replacement = editor.schema.nodes.quote.create(null, [
    editor.schema.nodes.paragraph.create(null, editor.schema.text('Nouveau contenu')),
    editor.schema.nodes.quoteAttribution.create(),
  ])
  await applyObjectEdit(
    editor,
    editor.state.tr.replaceWith(0, node.nodeSize, replacement),
    undefined,
    { focus: false },
  )
  expect(editor.state.selection.$from.parent.textContent).toBe('Nouveau contenu')
  expect(editor.state.selection.$from.parentOffset).toBe(0)
})
it('removing a widget preserves the explicit destination after deletion', async () => {
  const editor = await create()
  editor.commands.setContent('<ol><li><p>À supprimer</p></li></ol><p>Après</p>')
  editor.commands.setTextSelection(6)
  const node = editor.state.doc.firstChild!
  const transaction = editor.state.tr.delete(0, node.nodeSize)
  const selection = transaction.selection.toJSON()
  await applyObjectEdit(editor, transaction, undefined, { focus: false })
  expect(editor.state.doc.firstChild!.textContent).toBe('Après')
  expect(editor.state.selection.toJSON()).toEqual(selection)
})

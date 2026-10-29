import { afterEach, expect, it } from 'vitest'
import { AllSelection, NodeSelection, TextSelection } from '@tiptap/pm/state'
import { ySyncPluginKey } from '@tiptap/y-tiptap'
import { CellSelection } from '@tiptap/pm/tables'
import { createRichZone } from '../src/rich-zone'
import { bindEditorTarget } from '../src/editor-targets'
import { editingView, preserveTypingStyles, type EditingView } from '../src/editing-context'
import { applyFormatting } from '../src/formatting'
import { Provider, type HistoryState, type Transport } from '../src/provider'
import * as Y from 'yjs'
import type { Editor } from '@tiptap/core'
const editors: Editor[] = []
const providers: Provider[] = []
afterEach(() => {
  editors.splice(0).forEach((e) => e.destroy())
  providers.splice(0).forEach((p) => p.destroy())
})
async function editor() {
  const e = createRichZone({
    profile: 'document',
    label: 'Contexte',
    content: {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'A😀', marks: [{ type: 'bold' }] },
            { type: 'footnote', attrs: { text: 'Une note' } },
            { type: 'text', text: 'é日本', marks: [{ type: 'italic' }] },
          ],
        },
        { type: 'paragraph', content: [{ type: 'text', text: 'Suite' }] },
      ],
    },
  })
  editors.push(e)
  if (!e.isInitialized) await new Promise<void>((resolve) => e.once('create', () => resolve()))
  bindEditorTarget(e, { kind: 'body' })
  return e
}
const point = (path: number[], offset: number) => ({ zone: { kind: 'body', path }, offset })
it('preserves backward multizone selections, Unicode and atom widths', async () => {
  const e = await editor()
  e.view.dispatch(e.state.tr.setSelection(TextSelection.create(e.state.doc, 12, 2)))
  expect(editingView(e)?.selection).toEqual({
    kind: 'text',
    anchor: point([1], 2),
    head: point([0], 1),
  })
  e.commands.setTextSelection(5) // after A😀 and the one-unit note, before é
  expect(editingView(e)?.selection).toEqual({
    kind: 'text',
    anchor: point([0], 4),
    head: point([0], 4),
  })
})
it('distinguishes inherited marks from explicitly stopped typing styles', async () => {
  const e = await editor()
  e.commands.setTextSelection(2)
  expect(editingView(e)?.typing_styles).toEqual([{ type: 'bold' }])
  await applyFormatting(e, 'text.clear', { focus: false })
  expect(editingView(e)?.typing_styles).toEqual([])
  e.commands.toggleItalic()
  e.commands.setLink({ href: 'https://example.org' })
  expect(editingView(e)?.typing_styles).toEqual([
    { type: 'link', url: 'https://example.org' },
    { type: 'italic' },
  ])
})
it('retains structural object, whole-zone and table cell selections', async () => {
  const e = await editor()
  e.view.dispatch(e.state.tr.setSelection(NodeSelection.create(e.state.doc, 4)))
  expect(editingView(e)).toEqual({
    selection: { kind: 'node', target: { root: { kind: 'body' }, path: [0, 1] } },
    typing_styles: [],
  })
  e.view.dispatch(e.state.tr.setSelection(new AllSelection(e.state.doc)))
  expect(editingView(e)?.selection).toEqual({ kind: 'all', root: { kind: 'body' } })
  e.commands.setContent({
    type: 'doc',
    content: [
      {
        type: 'tableFigure',
        content: [
          {
            type: 'table',
            content: [
              {
                type: 'tableRow',
                content: [
                  { type: 'tableCell', content: [{ type: 'paragraph' }] },
                  { type: 'tableCell', content: [{ type: 'paragraph' }] },
                ],
              },
            ],
          },
          { type: 'tableCaption' },
        ],
      },
      { type: 'paragraph' },
    ],
  })
  const cells: number[] = []
  e.state.doc.descendants((node, pos) => {
    if (node.type.name === 'tableCell') cells.push(pos)
  })
  e.view.dispatch(e.state.tr.setSelection(CellSelection.create(e.state.doc, cells[1], cells[0])))
  expect(editingView(e)?.selection).toEqual({
    kind: 'cells',
    anchor: { root: { kind: 'body' }, path: [0, 0, 0, 1] },
    head: { root: { kind: 'body' }, path: [0, 0, 0, 0] },
  })
})
it('addresses a rich header field by stable record identity', async () => {
  const e = await editor()
  bindEditorTarget(e, { kind: 'record', collection: 'localizations', id: 'fr', field: 'abstract' })
  e.commands.setTextSelection(2)
  expect(editingView(e)?.selection).toEqual({
    kind: 'text',
    anchor: {
      zone: { kind: 'record', collection: 'localizations', id: 'fr', field: 'abstract', path: [0] },
      offset: 1,
    },
    head: {
      zone: { kind: 'record', collection: 'localizations', id: 'fr', field: 'abstract', path: [0] },
      offset: 1,
    },
  })
})
const view = (offset = 0): EditingView => ({
  selection: { kind: 'text', anchor: point([0], offset) as any, head: point([0], offset) as any },
  typing_styles: [],
})
const history = (request: any): HistoryState => ({
  guard: { document: request.generation, revision: request.revision },
  selection: request.selection,
  typing_styles: request.typing_styles,
  undo_steps: 0,
  redo_steps: 0,
  pending: false,
})
function provider(invoke: Transport['invoke']) {
  const p = new Provider('context', { invoke } as Transport)
  providers.push(p)
  p.online = p.nativeEditing = true
  p.generation = 'one'
  return p
}
it('coalesces navigation privately without publication, dirty input or history', async () => {
  const calls: any[] = []
  const p = provider(async (command, args) => {
    calls.push({ command, args })
    return history(args!.request) as any
  })
  let changed = 0
  p.onChange = () => changed++
  const bytes = Y.encodeStateAsUpdate(p.doc)
  const owner = {}
  for (let i = 0; i < 20; i++) p.updateEditingContext(owner, () => view(i))
  await p.flushEditingContext()
  expect(calls).toHaveLength(1)
  expect(calls[0].command).toBe('set_editing_context')
  expect(calls[0].args.request.selection.head.offset).toBe(19)
  expect(calls[0].args.request.update).toBeUndefined()
  expect(
    Y.equalSnapshots(
      Y.decodeSnapshot(new Uint8Array(calls[0].args.request.frontier)),
      Y.snapshot(p.doc),
    ),
  ).toBe(true)
  expect(Y.encodeStateAsUpdate(p.doc)).toEqual(bytes)
  expect(changed).toBe(0)
  expect(p.historyState.typing_styles).toEqual([])
})
it('waits for typing confirmation and keeps preedit local until composition ends', async () => {
  let confirm!: () => void
  const calls: any[] = []
  const p = provider(async (command, args) => {
    calls.push({ command, args })
    if (command === 'relay') {
      await new Promise<void>((resolve) => {
        confirm = resolve
      })
      return {
        revision: 1,
        update: [],
        history: { undo_steps: 1, redo_steps: 0, pending: false },
      } as any
    }
    return history(args!.request) as any
  })
  const owner = {}
  const token = p.beginComposition()
  p.doc.getText('input').insert(0, '日本')
  p.updateEditingContext(owner, () => view(2))
  await p.flushEditingContext()
  expect(calls).toEqual([])
  p.endComposition(token)
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(calls.map((c) => c.command)).toEqual(['relay'])
  confirm()
  await p.flush(false)
  expect(calls.map((c) => c.command)).toEqual(['relay', 'set_editing_context'])
  expect(calls[1].args.request.revision).toBe(1)
})
it('ignores late responses from another zone and never quarantines a refused navigation', async () => {
  let answer!: (value: any) => void
  const calls: any[] = []
  const p = provider(async (command, args) => {
    calls.push({ command, args })
    if (calls.length === 1)
      return await new Promise((resolve) => {
        answer = resolve
      })
    if (calls.length === 2) throw JSON.stringify({ code: 'stale_revision' })
    return history(args!.request) as any
  })
  p.updateEditingContext({}, () => view(1))
  await new Promise((resolve) => setTimeout(resolve, 0))
  p.updateEditingContext({}, () => ({ ...view(3), typing_styles: [{ type: 'italic' }] }))
  answer({
    ...history(calls[0].args.request),
    availability: { 'text.bold': { enabled: false, reason: 'body_required' } },
  })
  await p.flushEditingContext()
  expect(p.historyState.selection).toBeUndefined()
  expect(p.historyState.availability).toBeUndefined()
  expect(calls).toHaveLength(2)
  expect(p.rejected).toBeNull()
  expect(p.online).toBe(true)
  p.updateEditingContext({}, () => view(4))
  await p.flushEditingContext()
  expect((p.historyState.selection as any).head.offset).toBe(4)
})

it('keeps explicit on/off typing styles across remote transactions without capturing them', async () => {
  const e = await editor()
  e.registerPlugin(preserveTypingStyles())
  e.commands.setTextSelection(2)
  e.commands.toggleItalic()
  const styles = editingView(e)!.typing_styles
  e.view.dispatch(
    e.state.tr.insertText('Distant', 10).setMeta(ySyncPluginKey, { isChangeOrigin: true }),
  )
  expect(editingView(e)!.typing_styles).toEqual(styles)
  await applyFormatting(e, 'text.clear', { focus: false })
  e.view.dispatch(
    e.state.tr.insertText('Autre', 10).setMeta(ySyncPluginKey, { isChangeOrigin: true }),
  )
  expect(e.state.storedMarks).toEqual([])
  expect(editingView(e)!.typing_styles).toEqual([])
  e.commands.toggleBold()
  expect(editingView(e)!.typing_styles).toEqual([{ type: 'bold' }])
})

it('a checkpoint rechecks IME after waiting for a private context response', async () => {
  let answer!: () => void
  const p = provider(async (_command, args) => {
    await new Promise<void>((resolve) => {
      answer = resolve
    })
    return history(args!.request) as any
  })
  p.updateEditingContext({}, () => view())
  const checkpoint = p.flush(false)
  const assertion = expect(checkpoint).rejects.toThrow('saisie en cours')
  await new Promise((resolve) => setTimeout(resolve, 0))
  p.beginComposition()
  answer()
  await assertion
})
it('a checkpoint awaits text typed while its private context is in flight', async () => {
  let answer!: () => void
  const calls: string[] = []
  const p = provider(async (command, args) => {
    calls.push(command)
    if (calls.length === 1)
      await new Promise<void>((resolve) => {
        answer = resolve
      })
    if (command === 'relay')
      return {
        revision: 1,
        history: { undo_steps: 1, redo_steps: 0, pending: false },
        update: [],
      } as any
    return history(args!.request) as any
  })
  p.updateEditingContext({}, () => view())
  const checkpoint = p.flush(false)
  await new Promise((resolve) => setTimeout(resolve, 0))
  p.doc.getText('input').insert(0, 'Tardif')
  answer()
  await checkpoint
  expect(p.revision).toBe(1)
  expect(calls).toContain('relay')
  expect(calls.at(-1)).toBe('set_editing_context')
})

it('reports the styles PM will use to replace a selection in either direction', async () => {
  const e = await editor()
  for (const [anchor, head] of [
    [2, 12],
    [12, 2],
  ]) {
    e.view.dispatch(e.state.tr.setSelection(TextSelection.create(e.state.doc, anchor, head)))
    const replacement = e.state.tr.insertText('X').doc.nodeAt(2)!
    expect(editingView(e)!.typing_styles).toEqual(
      replacement.marks.map((mark) => ({ type: mark.type.name })),
    )
    expect(editingView(e)!.typing_styles).toEqual([{ type: 'bold' }])
  }
})

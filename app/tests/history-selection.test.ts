import { Editor } from '@tiptap/core'
import { afterEach, expect, it } from 'vitest'
import { richZoneExtensions } from '../src/rich-zone'
import { bindEditorTarget } from '../src/editor-targets'
import { restoreHistorySelection } from '../src/history-selection'
import type { HistoryState } from '../src/provider'
const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((e) => e.destroy()))
function create() {
  const e = new Editor({
    element: document.createElement('div'),
    extensions: richZoneExtensions('document'),
    content: '<p>A😀<span data-footnote="Note"></span>B</p>',
  })
  editors.push(e)
  e.commands.setTextSelection(1)
  bindEditorTarget(e, { kind: 'body' })
  return e
}
function view(): HistoryState {
  const point = (offset: number) => ({ zone: { kind: 'body' as const, path: [0] }, offset })
  return {
    undo_steps: 1,
    redo_steps: 0,
    pending: false,
    selection: { kind: 'text', anchor: point(4), head: point(3) },
    typing_styles: [],
  }
}
it('restaure la direction, les offsets UTF-16 autour des badges et l’arrêt explicite du gras sans déplacer le focus', () => {
  const e = create()
  e.commands.setBold()
  expect(restoreHistorySelection(e, view())).toBe(true)
  expect(e.state.selection.anchor).toBe(5)
  expect(e.state.selection.head).toBe(4)
  const collapsed = view()
  if (collapsed.selection?.kind === 'text') collapsed.selection.anchor = collapsed.selection.head
  expect(restoreHistorySelection(e, collapsed)).toBe(true)
  expect(e.state.storedMarks).toEqual([])
  expect(e.isFocused).toBe(false)
})
it('ne devine jamais un champ ou une position de remplacement si la cible est perdue ou absente', () => {
  const e = create(),
    original = e.state.selection
  const invalid = view()
  for (const selection of [
    { ...invalid, selection_lost: true },
    { ...invalid, selection: null },
    {
      ...invalid,
      selection: {
        kind: 'node' as const,
        target: {
          root: { kind: 'record' as const, collection: 'localizations', id: 'fr', field: 'title' },
          path: [0],
        },
      },
    },
    {
      ...invalid,
      selection: {
        kind: 'text' as const,
        anchor: { zone: { kind: 'body' as const, path: [30] }, offset: 0 },
        head: { zone: { kind: 'body' as const, path: [0] }, offset: 90 },
      },
    },
  ])
    expect(restoreHistorySelection(e, selection)).toBe(false)
  expect(e.state.selection.eq(original)).toBe(true)
})

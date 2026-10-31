import { beforeEach, afterEach, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import { comparisonView } from '../src/comparison-view'
import { comparisonAPI } from '../src/comparison-api'
import { versionDialog } from '../src/version-dialog'
import type { ComparisonAPI, ComparisonState } from '../src/comparison-types'
import type { Provider, Transport } from '../src/provider'
import type { Catalogue } from '../src/shell'
import type { VersionAPI } from '../src/version-types'
import articleBadges from '../../fixtures/comparison-article-badges.json'
const originalShowModal = HTMLDialogElement.prototype.showModal,
  originalClose = HTMLDialogElement.prototype.close

const catalogue = { locale: 'fr', types: [] } as unknown as Catalogue
const snapshot = (text: string) => ({
  state: {
    document: {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'Anchor' }] },
        { type: 'paragraph', content: [{ type: 'text', text }] },
      ],
    },
    settings: {
      metadata: { type: 'courrier', fields: { options: {} } },
      template: '',
      modelId: 'personal',
    },
  },
  resources: [],
})
function fixture(source: 'before' | 'after' = 'before') {
  let state: ComparisonState = {
    id: 'review',
    before: snapshot('Old'),
    after: snapshot('New'),
    report: {
      guard: { document: 'doc', revision: 2 },
      source,
      stale: false,
      groups: [0, 1].map((id) => ({
        id,
        section: 'body',
        before: { start: id * 2, end: id * 2 + 1 },
        after: { start: id * 2, end: id * 2 + 1 },
        linked: false,
        decision: 'pending',
        before_highlights: [{ path: [0, 0], from: 0, to: 3 }],
        after_highlights: [{ path: [0, 0], from: 0, to: 3 }],
        changes: [
          { kind: 'delete', text: 'Old' },
          { kind: 'insert', text: 'New' },
        ],
      })),
    },
  }
  let listener: () => void = () => {}
  const api: ComparisonAPI = {
    open: vi.fn(async () => structuredClone(state)),
    read: vi.fn(async () => structuredClone(state)),
    close: vi.fn(async () => {}),
    preview: vi.fn(async () => ({ pages: [] })),
    decide: vi.fn(async (_id, ids, action) => {
      state.report.guard.revision++
      state.report.groups.forEach((g) => {
        if (ids.includes(g.id)) g.decision = action === 'apply' ? 'applied' : 'refused'
      })
      return structuredClone(state)
    }),
    listen: vi.fn(async (fn) => {
      listener = fn
      return vi.fn()
    }),
  }
  return {
    api,
    state,
    change(fn: (s: ComparisonState) => void) {
      fn(state)
      listener()
    },
  }
}
const disposers: (() => void)[] = []
beforeEach(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn()
})
afterEach(() => {
  disposers.splice(0).forEach((fn) => fn())
  document.body.replaceChildren()
  HTMLDialogElement.prototype.showModal = originalShowModal
  HTMLDialogElement.prototype.close = originalClose
  vi.restoreAllMocks()
})
async function tick() {
  for (let i = 0; i < 12; i++) await Promise.resolve()
}
function button(root: Element, text: string) {
  return [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (b) => b.textContent === text,
  )!
}
it.each(['before', 'after'] as const)(
  'keeps frozen content and independent decisions for source %s',
  async (source) => {
    const f = fixture(source),
      error = vi.fn()
    const view = comparisonView({ state: structuredClone(f.state), api: f.api, catalogue, error })
    disposers.push(view.destroy)
    document.body.append(view.element)
    await tick()
    expect(view.element.querySelectorAll('del')).toHaveLength(2)
    expect(view.element.querySelectorAll('ins')).toHaveLength(2)
    const apply =
      source === 'before' ? 'Reprendre ce passage de la version' : 'Appliquer cette modification'
    button(view.element, apply).click()
    await tick()
    expect(f.api.decide).toHaveBeenCalledExactlyOnceWith('review', [0], 'apply')
    expect(view.element.textContent).toContain(
      source === 'before' ? 'Repris dans le document' : 'Appliqué au document',
    )
    expect(view.element.querySelector('del')!.textContent).toBe('Old')
    expect(view.element.querySelector('ins')!.textContent).toBe('New')
    expect(button(view.element, apply).disabled).toBe(false)
    button(view.element, source === 'before' ? 'Conserver le passage actuel' : 'Refuser').click()
    await tick()
    expect(f.api.decide).toHaveBeenLastCalledWith('review', [1], 'refuse')
    expect(error).not.toHaveBeenCalled()
  },
)
it('blocks pending applications after an external edit but preserves decisions and refuses safely', async () => {
  const f = fixture(),
    view = comparisonView({
      state: structuredClone(f.state),
      api: f.api,
      catalogue,
      error: vi.fn(),
    })
  disposers.push(view.destroy)
  document.body.append(view.element)
  await tick()
  button(view.element, 'Reprendre ce passage de la version').click()
  await tick()
  f.change((s) => (s.report.stale = true))
  await tick()
  expect(view.element.querySelector('.comparison-notice')!.hasAttribute('hidden')).toBe(false)
  expect(button(view.element, 'Reprendre ce passage de la version').disabled).toBe(true)
  expect(button(view.element, 'Reprendre le reste').disabled).toBe(true)
  expect(view.element.textContent).toContain('Repris dans le document')
  expect(button(view.element, 'Conserver le passage actuel').disabled).toBe(false)
})
it('does not expose decisions in historical read-only pairs or completed proposals', () => {
  const f = fixture()
  f.state.report.read_only = true
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(view.destroy)
  document.body.append(view.element)
  expect(view.element.querySelectorAll('.comparison-decisions button')).toHaveLength(0)
  expect(button(view.element, 'Reprendre le reste').hidden).toBe(true)
})
it('preserves linked groups and isolates late responses from a destroyed comparison', async () => {
  const f = fixture()
  f.state.report.groups[0].linked = true
  let finish!: (s: ComparisonState) => void
  vi.mocked(f.api.read).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve
      }),
  )
  const error = vi.fn(),
    view = comparisonView({ state: f.state, api: f.api, catalogue, error })
  document.body.append(view.element)
  await tick()
  expect(view.element.textContent).toContain('Éléments liés · une décision')
  f.change(() => {})
  view.destroy()
  finish(f.state)
  await tick()
  expect(error).not.toHaveBeenCalled()
})
it('sends only the review ID and group IDs, flushes confirmed writing without committing forms, and blocks IME or replacement', async () => {
  const invoke = vi.fn(async () => ({})),
    provider = {
      generation: 'doc',
      revision: 2,
      composing: false,
      flush: vi.fn(async () => {}),
      commitCommand: vi.fn(async (run: () => Promise<unknown>) => run()),
    }
  const api = comparisonAPI(
    provider as unknown as Provider,
    { invoke, listen: vi.fn(async () => () => {}) } as Transport,
  )
  await api.decide('trusted', [1], 'apply')
  expect(provider.flush).toHaveBeenCalledWith(false)
  expect(invoke).toHaveBeenLastCalledWith('comparison_decide', {
    id: 'trusted',
    groups: [1],
    action: 'apply',
  })
  provider.composing = true
  await expect(api.decide('trusted', [1], 'apply')).rejects.toThrow()
  expect(invoke).toHaveBeenCalledTimes(1)
  provider.composing = false
  provider.generation = 'other'
  await expect(api.open({ kind: 'version', id: 'old' })).rejects.toThrow()
  expect(invoke).toHaveBeenCalledTimes(1)
})
it('captures the current comparison once across tab changes and closes a delayed opening without publishing it', async () => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '')
  }
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open')
  }
  const f = fixture(),
    record = {
      id: 'v',
      created_at: '2026-10-31T10:00:00Z',
      reason: 'manual' as const,
      state: 'hash',
      details: { name: 'v1', tags: [], description: null, creator: null },
    }
  const api = {
    comparison: f.api,
    read: vi.fn(async () => f.state.before),
    preview: vi.fn(async () => ({ pages: [] })),
  } as unknown as VersionAPI
  const viewer = versionDialog({
    record,
    index: { project_id: 'project', history: 'index', limit: 100, versions: [record] },
    api,
    catalogue,
    returnFocus: () => undefined,
    changed: vi.fn(),
  })
  disposers.push(viewer.destroy)
  await tick()
  button(document.body, 'Comparer').click()
  await tick()
  button(document.body, 'Contenu').click()
  await tick()
  button(document.body, 'Comparer').click()
  await tick()
  expect(f.api.open).toHaveBeenCalledTimes(1)
  expect(document.querySelectorAll('.comparison')).toHaveLength(1)
  viewer.destroy()
  expect(f.api.close).toHaveBeenCalledExactlyOnceWith('review')
})

it.each([false, true])(
  'installs the publication without an event, and ignores a response for a replaced document (%s)',
  async (replaced) => {
    const draft = new Y.Doc(),
      confirmed = new Y.Doc()
    confirmed.getText('test').insert(0, 'Confirmed')
    const update = [...Y.encodeStateAsUpdate(confirmed)]
    const provider = {
      doc: draft,
      generation: 'doc',
      revision: 2,
      composing: false,
      flush: vi.fn(async () => {}),
      commitCommand: vi.fn(async (run: () => Promise<unknown>) => run()),
      refreshHistory: vi.fn(async () => {}),
      addResource: vi.fn(),
    }
    const f = fixture(),
      invoke = vi.fn(async () => {
        if (replaced) provider.generation = 'new-document'
        return {
          ...f.state,
          publication: { guard: { document: 'doc', revision: 3 }, update, resources: [] },
        }
      })
    const api = comparisonAPI(
      provider as unknown as Provider,
      { invoke, listen: vi.fn(async () => () => {}) } as Transport,
    )
    await api.decide('review', [0], 'apply')
    expect(draft.getText('test').toString()).toBe(replaced ? '' : 'Confirmed')
    expect(provider.refreshHistory).toHaveBeenCalledTimes(replaced ? 0 : 1)
    expect(provider.commitCommand).toHaveBeenCalledWith(expect.any(Function), false)
    draft.destroy()
    confirmed.destroy()
  },
)

it('navigates groups with up/down without applying a modification or leaving safe controls', async () => {
  const f = fixture(),
    view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(view.destroy)
  document.body.append(view.element)
  const next = view.element.querySelector<HTMLButtonElement>('[data-next]')!
  next.focus()
  next.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }),
  )
  expect(view.element.querySelector('[data-active=true]')?.getAttribute('data-group')).toBe('1')
  expect(document.activeElement).toBe(view.element.querySelector('[data-next]'))
  expect(f.api.decide).not.toHaveBeenCalled()
})

it('shows general options and the archived signature in documentary groups', async () => {
  const create = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:signature'),
    revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  const f = fixture()
  f.state.report.groups = f.state.report.groups.slice(0, 1)
  f.state.report.groups[0].section = 'document'
  f.state.before.state.settings.metadata.fields = {
    options: { paper: 'a4', landscape: false, language: 'fr' },
    signature: 'images/signature.svg',
  } as typeof f.state.before.state.settings.metadata.fields
  f.state.before.resources = [
    {
      path: 'images/signature.svg',
      data: btoa('<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>'),
    },
  ] as typeof f.state.before.resources
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  document.body.append(view.element)
  expect(view.element.textContent).toContain('A4')
  expect(view.element.textContent).toContain('Portrait')
  expect(view.element.querySelector('.version-signature')?.getAttribute('src')).toBe(
    'blob:signature',
  )
  view.destroy()
  expect(revoke).toHaveBeenCalledWith('blob:signature')
  expect(create).toHaveBeenCalledTimes(1)
})

it('marks doit → peut and xx beside a citation without flattening badges or inline styles', () => {
  const f = fixture()
  f.state.before.state.document = articleBadges.before
  f.state.after.state.document = articleBadges.after
  const oldText = articleBadges.before.content[0].content![2].text!
  const newText = articleBadges.after.content[0].content![2].text!
  f.state.report.groups = [
    {
      ...f.state.report.groups[0],
      before: { start: 0, end: 2 },
      after: { start: 0, end: 2 },
      before_highlights: [
        { path: [0, 2], from: oldText.indexOf('doit'), to: oldText.indexOf('doit') + 4 },
      ],
      after_highlights: [
        { path: [0, 2], from: newText.indexOf('peut'), to: newText.indexOf('peut') + 4 },
        { path: [0, 2], from: newText.indexOf('xx'), to: newText.indexOf('xx') + 3 },
      ],
    },
  ]
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(view.destroy)
  const [before, after] = view.element.querySelectorAll('.comparison-side')
  expect(before.querySelector('del')?.textContent).toBe('doit')
  expect([...after.querySelectorAll('ins')].map((n) => n.textContent)).toEqual(['peut', 'xx '])
  for (const side of [before, after]) {
    expect(side.querySelector('[data-citation]')).not.toBeNull()
    expect(side.querySelector('[data-footnote]')?.textContent).toBe('Note 1')
    expect(side.querySelector('em')?.textContent).toBe('suppose')
    expect(side.querySelector('strong')?.textContent).toBe('présentation')
  }
  expect(f.state.before.state.document).toEqual(articleBadges.before)
  expect(f.state.after.state.document).toEqual(articleBadges.after)
})

it('marks changed text and notes inside a widget while preserving its structure and styles', () => {
  const f = fixture()
  const document = (word: string, note: string) => ({
    type: 'doc',
    content: [
      {
        type: 'bulletList',
        content: [
          {
            type: 'listItem',
            content: [
              {
                type: 'paragraph',
                content: [
                  { type: 'text', text: '😀 Un ' },
                  { type: 'text', text: word, marks: [{ type: 'bold' }] },
                  { type: 'footnote', attrs: { text: note } },
                ],
              },
            ],
          },
        ],
      },
    ],
  })
  f.state.before.state.document = document('ancien', 'Avant')
  f.state.after.state.document = document('nouveau', 'Après')
  f.state.report.groups = [
    {
      ...f.state.report.groups[0],
      before_highlights: [
        { path: [0, 0, 0, 1], from: 0, to: 6 },
        { path: [0, 0, 0, 2], from: 0, to: 1 },
      ],
      after_highlights: [
        { path: [0, 0, 0, 1], from: 0, to: 7 },
        { path: [0, 0, 0, 2], from: 0, to: 1 },
      ],
    },
  ]
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(view.destroy)
  expect(view.element.querySelectorAll('ul li')).toHaveLength(2)
  expect(view.element.querySelector('strong del')?.textContent).toBe('ancien')
  expect(view.element.querySelector('strong ins')?.textContent).toBe('nouveau')
  expect(view.element.querySelector('del .footnote-ref')?.getAttribute('title')).toBe('Avant')
  expect(view.element.querySelector('ins .footnote-ref')?.textContent).toBe('Note 1')
})

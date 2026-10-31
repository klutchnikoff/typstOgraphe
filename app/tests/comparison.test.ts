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
import { availableIcon } from '../src/icons'
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
      context: source === 'before' ? 'version' : 'proposal',
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
    prepare: vi.fn(async () => ({
      ...structuredClone(state),
      id: 'composed',
      report: {
        ...structuredClone(state.report),
        context: 'composition' as const,
        source: 'after' as const,
      },
    })),
    apply: vi.fn(async (_id, choices) => {
      state.report.guard.revision++
      state.report.closed = true
      state.report.applied = true
      state.report.groups.forEach((g) => {
        g.decision = choices[g.id] === state.report.source ? 'applied' : 'refused'
      })
      return {
        ...structuredClone(state),
      }
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
      state = structuredClone(state)
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
function validate(root: Element) {
  return root.querySelector<HTMLButtonElement>('[data-action="validate-comparison"]')!
}
function button(root: Element, text: string) {
  return [...root.querySelectorAll<HTMLButtonElement>('button')].find(
    (b) => b.textContent === text,
  )!
}
it.each(['before', 'after'] as const)(
  'stages exclusive red/green choices for %s and publishes only once',
  async (source) => {
    const f = fixture(source),
      error = vi.fn()
    const view = comparisonView({ state: f.state, api: f.api, catalogue, error })
    disposers.push(view.destroy)
    document.body.append(view.element)
    await tick()
    const radio = (group: number, side: string) =>
      view.element.querySelector<HTMLButtonElement>(
        `[data-group="${group}"] [data-side="${side}"]`,
      )!
    expect(view.element.querySelectorAll('.comparison-group > header [role=radio]')).toHaveLength(4)
    expect(view.element.querySelector('.comparison-group > header')!.textContent).not.toContain(
      'Choix provisoire',
    )
    expect(radio(0, 'after').getAttribute('aria-checked')).toBe('true')
    expect(radio(0, 'after').tabIndex).toBe(0)
    const iconPath = (name: string) => {
      const asset = availableIcon(name)
      expect(asset).toBeDefined()
      return new DOMParser()
        .parseFromString(asset!, 'image/svg+xml')
        .querySelector('path')!
        .getAttribute('d')
    }
    expect(radio(0, 'after').querySelector('path')?.getAttribute('d')).toBe(iconPath('check'))
    expect(radio(0, 'before').querySelector('path')?.getAttribute('d')).toBe(iconPath('x'))
    radio(0, 'before').click()
    expect(radio(0, 'before').getAttribute('aria-checked')).toBe('true')
    expect(radio(0, 'after').getAttribute('aria-checked')).toBe('false')
    expect(radio(0, 'before').querySelector('path')?.getAttribute('d')).toBe(iconPath('check'))
    expect(radio(0, 'after').querySelector('path')?.getAttribute('d')).toBe(iconPath('x'))
    expect(f.api.apply).not.toHaveBeenCalled()
    radio(0, 'before').focus()
    radio(0, 'before').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }),
    )
    expect(radio(0, 'after').getAttribute('aria-checked')).toBe('true')
    expect(document.activeElement).toBe(radio(0, 'after'))
    radio(0, 'before').click()
    validate(view.element).click()
    await tick()
    expect(f.api.apply).toHaveBeenCalledExactlyOnceWith('review', ['before', 'after'])
    expect(validate(view.element).disabled).toBe(true)
    expect(radio(0, 'before').disabled).toBe(true)
    expect(view.element.querySelector('del')!.textContent).toBe('Old')
    expect(view.element.querySelector('ins')!.textContent).toBe('New')
    expect(error).not.toHaveBeenCalled()
  },
)
it.each(['before', 'after'] as const)(
  'selects every group on %s without publishing, including linked groups, until Apply',
  async (source) => {
    const f = fixture(source)
    f.state.report.groups[1].linked = true
    const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
    disposers.push(view.destroy)
    document.body.append(view.element)
    await tick()
    const select = (side: string) =>
      view.element.querySelector<HTMLButtonElement>(`[data-action="select-all-${side}"]`)!
    select('before').focus()
    select('before').click()
    expect(document.activeElement).toBe(select('before'))
    expect(view.element.querySelectorAll('[data-side="before"][aria-checked="true"]')).toHaveLength(
      2,
    )
    expect(f.state.choices).toEqual(['before', 'before'])
    await tick()
    expect(
      [...view.element.querySelectorAll<HTMLButtonElement>('[role=radio]')].every(
        (b) => b.tabIndex === (b.getAttribute('aria-checked') === 'true' ? 0 : -1),
      ),
    ).toBe(true)
    select('before').dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'ArrowRight',
        bubbles: true,
        cancelable: true,
      }),
    )
    expect(document.activeElement).toBe(select('after'))
    select('after').click()
    expect(f.state.choices).toEqual(['after', 'after'])
    select('before').click()
    expect(f.api.apply).not.toHaveBeenCalled()
    validate(view.element).click()
    expect(select('before').disabled).toBe(true)
    expect(select('after').disabled).toBe(true)
    await tick()
    expect(f.api.apply).toHaveBeenCalledExactlyOnceWith('review', ['before', 'before'])
    expect(select('before').disabled).toBe(true)
    expect(select('after').disabled).toBe(true)
  },
)
it('disables bulk choices without differences while keeping identical historical states applicable', () => {
  const f = fixture()
  f.state.report.groups = []
  f.state.report.context = 'historical'
  const view = comparisonView({
    state: f.state,
    api: f.api,
    catalogue,
    error: vi.fn(),
    prepared: vi.fn(),
  })
  disposers.push(view.destroy)
  expect(
    [...view.element.querySelectorAll<HTMLButtonElement>('[data-action^="select-all-"]')].every(
      (b) => b.disabled,
    ),
  ).toBe(true)
  expect(validate(view.element).disabled).toBe(false)
})
it('preserves provisional choices after a refused publication and an external edit', async () => {
  const f = fixture(),
    error = vi.fn()
  vi.mocked(f.api.apply).mockRejectedValueOnce(new Error('invalid candidate'))
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error })
  disposers.push(view.destroy)
  document.body.append(view.element)
  await tick()
  view.element.querySelector<HTMLButtonElement>('[data-group="0"] [data-side="before"]')!.click()
  validate(view.element).click()
  await tick()
  expect(error).toHaveBeenCalledOnce()
  expect(
    view.element
      .querySelector('.comparison-choice[data-side="before"]')
      ?.getAttribute('aria-checked'),
  ).toBe('true')
  f.change((state) => {
    state.report.stale = true
  })
  await tick()
  expect(validate(view.element).disabled).toBe(true)
  expect(view.element.querySelector('.comparison-notice')!.hasAttribute('hidden')).toBe(false)
})
it('disables composition once the comparison is finished', () => {
  const f = fixture()
  f.state.report.closed = true
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(view.destroy)
  expect(validate(view.element).disabled).toBe(true)
  expect(
    [...view.element.querySelectorAll<HTMLButtonElement>('[role=radio]')].every((b) => b.disabled),
  ).toBe(true)
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
it('applies without a destination or save acknowledgement, flushes confirmed writing without committing forms, and blocks IME or replacement', async () => {
  const f = fixture(),
    invoke = vi.fn(async () => f.state),
    provider = {
      generation: 'doc',
      revision: 2,
      composing: false,
      flush: vi.fn(async () => {}),
      confirmSaved: vi.fn(),
      commitCommand: vi.fn(async (run: () => Promise<unknown>) => run()),
    }
  const api = comparisonAPI(
    provider as unknown as Provider,
    { invoke, listen: vi.fn(async () => () => {}) } as Transport,
  )
  await api.apply('trusted', ['after', 'before'])
  expect(provider.flush).toHaveBeenCalledWith(false)
  expect(provider.flush).toHaveBeenCalledTimes(1)
  expect(invoke).toHaveBeenCalledExactlyOnceWith('comparison_apply', {
    id: 'trusted',
    choices: ['after', 'before'],
  })
  expect(provider.confirmSaved).not.toHaveBeenCalled()
  provider.composing = true
  await expect(api.apply('trusted', ['after', 'before'])).rejects.toThrow()
  expect(invoke).toHaveBeenCalledTimes(1)
  provider.composing = false
  provider.generation = 'other'
  await expect(api.open({ kind: 'version', id: 'old' })).rejects.toThrow()
  expect(invoke).toHaveBeenCalledTimes(1)
})
it('keeps an empty composition delta from creating a Yjs edit or marking the document saved', async () => {
  const doc = new Y.Doc()
  const provider = {
    doc,
    generation: 'doc',
    revision: 2,
    composing: false,
    flush: vi.fn(async () => {}),
    commitCommand: vi.fn(async (run: () => Promise<unknown>) => run()),
    confirmSaved: vi.fn(),
    refreshHistory: vi.fn(async () => {}),
  }
  const f = fixture(),
    invoke = vi.fn(async () => ({
      ...f.state,
      publication: { guard: { document: 'doc', revision: 2 }, update: [], resources: [] },
    })),
    changed = vi.fn()
  doc.on('update', changed)
  const api = comparisonAPI(
    provider as unknown as Provider,
    { invoke, listen: vi.fn(async () => () => {}) } as Transport,
  )
  await api.apply('review', ['after'])
  expect(provider.confirmSaved).not.toHaveBeenCalled()
  expect(changed).not.toHaveBeenCalled()
  expect(provider.revision).toBe(2)
  doc.destroy()
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
  document.querySelector<HTMLButtonElement>('[data-group="0"] [data-side="before"]')!.click()
  button(document.body, 'Contenu').click()
  await tick()
  button(document.body, 'Comparer').click()
  await tick()
  expect(f.api.open).toHaveBeenCalledTimes(1)
  expect(document.querySelectorAll('.comparison')).toHaveLength(1)
  expect(
    document.querySelector('[data-group="0"] [data-side="before"]')?.getAttribute('aria-checked'),
  ).toBe('true')
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
      confirmSaved: vi.fn(),
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
    await api.apply('review', ['before', 'after'])
    expect(draft.getText('test').toString()).toBe(replaced ? '' : 'Confirmed')
    expect(provider.refreshHistory).toHaveBeenCalledTimes(replaced ? 0 : 1)
    expect(provider.confirmSaved).not.toHaveBeenCalled()
    expect(provider.commitCommand).toHaveBeenCalledWith(expect.any(Function))
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
  expect(f.api.apply).not.toHaveBeenCalled()
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

it('prepares historical choices without applying and releases a late preparation after closing', async () => {
  const f = fixture(),
    prepared = vi.fn()
  f.state.report.context = 'historical'
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn(), prepared })
  disposers.push(view.destroy)
  view.element.querySelector<HTMLButtonElement>('[data-group="0"] [data-side="before"]')!.click()
  validate(view.element).click()
  await tick()
  expect(f.api.prepare).toHaveBeenCalledExactlyOnceWith('review', ['before', 'after'])
  expect(f.api.apply).not.toHaveBeenCalled()
  expect(prepared).toHaveBeenCalledWith(expect.objectContaining({ id: 'composed' }))
  view.destroy()
  let finish!: (value: ComparisonState) => void
  vi.mocked(f.api.prepare).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve
      }),
  )
  const late = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn(), prepared })
  validate(late.element).click()
  late.destroy()
  finish({ ...f.state, id: 'late' })
  await tick()
  expect(f.api.close).toHaveBeenCalledWith('late')
  expect(prepared).toHaveBeenCalledTimes(1)
})

it('prepares privately without committing Yjs or forms, and closes a candidate for a replaced document', async () => {
  const f = fixture(),
    provider = {
      generation: 'doc',
      revision: 2,
      composing: false,
      flush: vi.fn(async () => {}),
      commitCommand: vi.fn(),
      confirmSaved: vi.fn(),
    },
    invoke = vi.fn(async (command: string) => {
      if (command === 'comparison_prepare') provider.generation = 'replacement'
      return { ...f.state, id: 'private' }
    })
  const api = comparisonAPI(
    provider as unknown as Provider,
    { invoke, listen: vi.fn() } as unknown as Transport,
  )
  await expect(api.prepare('review', ['before', 'after'])).rejects.toThrow()
  expect(provider.flush).toHaveBeenCalledExactlyOnceWith(false)
  expect(invoke).toHaveBeenCalledWith('comparison_close', { id: 'private' })
  expect(provider.commitCommand).not.toHaveBeenCalled()
  expect(provider.confirmSaved).not.toHaveBeenCalled()
})

it('announces full MCP rejection and preserves its terminal state when the view is reopened', async () => {
  const f = fixture('after')
  const view = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  document.body.append(view.element)
  view.element.querySelector<HTMLButtonElement>('[data-action="select-all-before"]')!.click()
  expect(validate(view.element).textContent).toBe('Refuser la proposition')
  expect(view.element.querySelector('[data-comparison-summary]')!.textContent).toContain(
    '2 changements écartés',
  )
  validate(view.element).click()
  await tick()
  expect(f.api.apply).toHaveBeenCalledExactlyOnceWith('review', ['before', 'before'])
  expect(view.element.querySelector('[data-comparison-summary]')!.textContent).toBe(
    'Proposition refusée. Le document est inchangé.',
  )
  view.destroy()
  const reopened = comparisonView({ state: f.state, api: f.api, catalogue, error: vi.fn() })
  disposers.push(reopened.destroy)
  expect(validate(reopened.element).disabled).toBe(true)
  expect(reopened.element.querySelector('[data-comparison-summary]')!.textContent).toBe(
    'Proposition refusée. Le document est inchangé.',
  )
})

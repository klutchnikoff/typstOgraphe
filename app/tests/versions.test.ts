import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { versionsPanel } from '../src/versions-panel'
import { versionsAPI } from '../src/versions-api'
import { initializeUI } from '../src/i18n'
import type { VersionAPI, VersionCatalogue } from '../src/version-types'
import type { Catalogue } from '../src/shell'
import type { Provider, Transport } from '../src/provider'
const originalShow = HTMLDialogElement.prototype.showModal,
  originalClose = HTMLDialogElement.prototype.close
beforeEach(() => {
  initializeUI({})
  HTMLElement.prototype.scrollIntoView = vi.fn()
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '')
  }
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open')
    this.dispatchEvent(new Event('close'))
  }
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  )
})
afterEach(() => {
  document.body.replaceChildren()
  HTMLDialogElement.prototype.showModal = originalShow
  HTMLDialogElement.prototype.close = originalClose
  vi.unstubAllGlobals()
})
const tick = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve()
}
const id = '25610e83-37e7-42a8-a794-a767a9479b90'
const record = {
  id,
  created_at: '2026-10-31T12:00:00Z',
  reason: 'manual' as const,
  state: 'sha256:state',
  details: {
    name: 'Version originale',
    tags: ['brouillon'],
    description: 'Une étape',
    creator: 'Camille',
  },
}
const index: VersionCatalogue = {
  project_id: 'project',
  history: 'index-1',
  limit: 100,
  versions: [record],
}
function fixture() {
  const root = document.createElement('div')
  document.body.append(root)
  const api: VersionAPI = {
    list: vi.fn(async () => structuredClone(index)),
    read: vi.fn(async () => ({
      state: {
        document: {
          type: 'doc',
          content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Contenu conservé' }] }],
        },
        settings: {
          metadata: { type: 'courrier', fields: { options: {} } },
          template: '',
          modelId: 'personal',
        },
      },
      resources: [],
    })),
    preview: vi.fn(async () => ({ pages: ['data:image/png;base64,aA=='] })),
    change: vi.fn(async (_history, action) =>
      action.kind === 'delete' ? { ...index, versions: [] } : null,
    ),
  }
  const catalogue = { locale: 'fr', types: [] } as unknown as Catalogue
  const panel = versionsPanel(root, api, catalogue, vi.fn())
  return { root, api, panel }
}
function key(root: Element, key: string) {
  root.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }))
}
it('consults immutable content, navigates tabs and dismisses a deletion before closing and returning focus', async () => {
  const { root, api, panel } = fixture()
  await tick()
  const row = root.querySelector<HTMLButtonElement>('[data-version]')!
  row.click()
  await tick()
  const dialog = document.querySelector('dialog')!
  expect(dialog.textContent).toContain('Contenu conservé')
  expect(dialog.querySelector('[contenteditable=true]')).toBeNull()
  const content = dialog.querySelector<HTMLButtonElement>('[data-view=content]')!
  content.focus()
  key(content, 'ArrowRight')
  await tick()
  expect(api.preview).toHaveBeenCalledWith(id)
  expect(dialog.querySelector('img')?.src).toContain('data:image/png')
  ;(dialog.querySelector('[aria-label="Supprimer cette version"]') as HTMLButtonElement).click()
  expect(dialog.querySelector('.properties')).not.toBeNull()
  const cancel = dialog.querySelector<HTMLButtonElement>('.properties .primary')!
  expect(cancel.textContent).toBe('Annuler')
  expect(document.activeElement).toBe(cancel)
  key(cancel, 'Escape')
  expect(dialog.querySelector('.properties')).toBeNull()
  expect(document.querySelector('dialog')).toBe(dialog)
  key(dialog, 'Escape')
  expect(document.querySelector('dialog')).toBeNull()
  expect(document.activeElement).toBe(row)
  expect(api.change).not.toHaveBeenCalled()
  panel.destroy()
})
it('keeps a relabelling draft and shows the failure when the index changed', async () => {
  const { root, api, panel } = fixture()
  await tick()
  root.querySelector<HTMLButtonElement>('[data-version]')!.click()
  await tick()
  ;(
    document.querySelector(
      '[aria-label="Modifier les informations de la version"]',
    ) as HTMLButtonElement
  ).click()
  const input = document.querySelector<HTMLInputElement>('[name=name]')!
  input.value = 'Nom saisi'
  vi.mocked(api.change).mockRejectedValueOnce('Index changed')
  key(input, 'Enter')
  await tick()
  expect(document.querySelector<HTMLInputElement>('[name=name]')?.value).toBe('Nom saisi')
  expect(document.querySelector('.properties [role=alert]')?.textContent).toContain('Index changed')
  expect(api.change).toHaveBeenCalledWith(
    'index-1',
    expect.objectContaining({
      kind: 'update',
      id,
      details: expect.objectContaining({ name: 'Nom saisi', creator: 'Camille' }),
    }),
  )
  panel.destroy()
})
it('deletion refreshes the list and returns focus to creation without changing the document', async () => {
  const { root, api, panel } = fixture()
  await tick()
  root.querySelector<HTMLButtonElement>('[data-version]')!.click()
  await tick()
  ;(document.querySelector('[aria-label="Supprimer cette version"]') as HTMLButtonElement).click()
  ;(document.querySelector('.properties .danger') as HTMLButtonElement).click()
  await tick()
  expect(api.change).toHaveBeenCalledWith('index-1', { kind: 'delete', id })
  expect(document.querySelector('dialog')).toBeNull()
  expect(root.querySelector('[data-version]')).toBeNull()
  expect(document.activeElement?.textContent).toContain('Créer une version')
  panel.destroy()
})
it('does not flush private document forms, send a cancelled save, or target a replacement document', async () => {
  const provider = {
    generation: 'original',
    revision: 4,
    flush: vi.fn(async () => {}),
  } as unknown as Provider
  const invoke = vi.fn(),
    saved = vi.fn()
  const api = versionsAPI({
    provider,
    transport: { invoke } as unknown as Transport,
    destination: async () => null,
    saved,
  })
  expect(await api.change('index', { kind: 'create', details: record.details })).toBeNull()
  expect(invoke).not.toHaveBeenCalled()
  await api.list()
  expect(provider.flush).toHaveBeenCalledWith(false)
  expect(invoke).toHaveBeenCalledWith('versions_list', {
    guard: { document: 'original', revision: 4 },
  })
  provider.generation = 'another'
  await expect(api.read(id)).rejects.toThrow('Le document a changé')
  expect(saved).not.toHaveBeenCalled()
})
it('disables creation at the quota while keeping deletion available', async () => {
  const { root, api, panel } = fixture()
  panel.destroy()
  vi.mocked(api.list).mockResolvedValue({ ...index, limit: 1 })
  const next = versionsPanel(
    root,
    api,
    { locale: 'fr', types: [] } as unknown as Catalogue,
    vi.fn(),
  )
  await tick()
  expect(root.querySelector<HTMLButtonElement>('.versions-panel-footer button')!.disabled).toBe(
    true,
  )
  root.querySelector<HTMLButtonElement>('[data-version]')!.click()
  await tick()
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Supprimer cette version"]')!.disabled,
  ).toBe(false)
  next.destroy()
})

it('does not bind a completed old save to a document opened while the request was pending', async () => {
  const provider = {
    generation: 'old',
    revision: 7,
    flush: vi.fn(async () => {}),
  } as unknown as Provider
  let resolve!: (value: unknown) => void
  const pending = new Promise((r) => (resolve = r)),
    invoke = vi.fn(() => pending),
    saved = vi.fn()
  const api = versionsAPI({
    provider,
    transport: { invoke } as unknown as Transport,
    destination: async () => undefined,
    saved,
  })
  const result = api.change('index', { kind: 'create', details: record.details })
  await tick()
  expect(invoke).toHaveBeenCalled()
  provider.generation = 'new'
  resolve({ ...index, path: 'old.typsto', revision: 7 })
  await expect(result).rejects.toThrow('Le document a changé')
  expect(saved).not.toHaveBeenCalled()
})

it('renders archived image bytes and citation labels without sharing the active editor resource cache', async () => {
  const { versionContent } = await import('../src/version-content')
  const create = vi
      .fn()
      .mockReturnValueOnce('blob:archived-image')
      .mockReturnValueOnce('blob:other-image'),
    revoke = vi.fn()
  vi.stubGlobal('URL', { createObjectURL: create, revokeObjectURL: revoke })
  const content = {
    state: {
      document: {
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [
              {
                type: 'citation',
                attrs: {
                  items: [{ file_id: 'books', key: 'key', supplement: '' }],
                  form: 'normal',
                },
              },
              { type: 'footnote', attrs: { text: 'Note conservée' } },
            ],
          },
          {
            type: 'figure',
            attrs: { src: 'images/shared.svg', alt: 'Ancienne image' },
            content: [],
          },
        ],
      },
      settings: {
        metadata: { type: 'article', fields: { options: {} } },
        template: '',
        modelId: 'working',
      },
    },
    references: [
      {
        file_id: 'books',
        file_name: 'books.bib',
        key: 'key',
        title: 'Une étude',
        authors: 'Martin',
        year: '2020',
        label: 'Martin, 2020',
      },
    ],
    resources: [
      {
        path: 'images/shared.svg',
        data: btoa('<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>'),
      },
    ],
  }
  const first = versionContent(content, { locale: 'fr', types: [] } as unknown as Catalogue)
  const second = versionContent(
    {
      ...content,
      resources: [
        {
          path: 'images/shared.svg',
          data: btoa('<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"/>'),
        },
      ],
    },
    { locale: 'fr', types: [] } as unknown as Catalogue,
  )
  expect(first.element.querySelector('img')?.getAttribute('src')).toBe('blob:archived-image')
  expect(second.element.querySelector('img')?.getAttribute('src')).toBe('blob:other-image')
  expect(first.element.querySelector('[data-citation]')?.textContent).toBe('(Martin, 2020)')
  expect(first.element.querySelector('[data-footnote]')?.getAttribute('title')).toBe(
    'Note conservée',
  )
  expect(first.element.querySelector('button')).toBeNull()
  first.destroy()
  expect(revoke).toHaveBeenCalledWith('blob:archived-image')
  expect(revoke).not.toHaveBeenCalledWith('blob:other-image')
  second.destroy()
  expect(revoke).toHaveBeenCalledWith('blob:other-image')
})

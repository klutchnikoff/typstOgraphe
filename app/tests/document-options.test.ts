import { afterEach, expect, it, vi } from 'vitest'
import { documentOptions } from '../src/document-options'
import type { Catalogue, DocumentType } from '../src/shell'
import type { MetadataCommands } from '../src/metadata-commands'
import type { Settings } from '../src/state'

const settings: Settings = {
  modelId: 'personal',
  template: 'Typst',
  metadata: {
    type: 'courrier',
    fields: { options: { paper: 'a3', landscape: true, language: 'fr' } },
  },
}
const kind: DocumentType = {
  id: 'courrier',
  label: 'Courrier',
  settingsHeading: 'Paramètres du courrier',
  canCreate: true,
  capabilities: [],
  fields: [],
  groups: [],
  metadata: settings.metadata,
  models: [
    {
      id: 'personal',
      label: 'Courrier personnel',
      description: '',
      source: 'Typst',
      fixedPage: { paper: 'a4', landscape: false },
    },
    { id: 'free', label: 'Modèle libre', description: '', source: 'Typst libre', fixedPage: null },
  ],
}
const catalogue: Catalogue = {
  locale: 'fr',
  messages: {},
  types: [kind],
  entries: [],
  papers: [
    { value: 'a4', label: 'A4' },
    { value: 'a3', label: 'A3' },
  ],
  languages: [
    { value: 'fr', label: 'Français' },
    { value: 'en', label: 'Anglais' },
  ],
}
afterEach(() => document.body.replaceChildren())
function mount() {
  const root = document.createElement('div'),
    set = vi.fn()
  let current = structuredClone(settings)
  document.body.append(root)
  const commands = {
    set,
    value: (_target: unknown, confirmed: unknown) => confirmed,
    error: () => undefined,
    recovering: () => false,
    confirm: set,
    watch: () => () => {},
    composing: () => {},
  } as unknown as MetadataCommands
  const rendered = documentOptions(root, kind, catalogue, current, {
    commands,
    read: () => current,
  })
  const view = {
    update: (value: Settings) => {
      current = value
      rendered.update(value)
    },
  }
  return { root, view, set }
}

it('les réglages imposés sont masqués sans écraser les choix conservés', () => {
  const { root, set } = mount()
  for (const name of ['paper', 'orientation']) {
    const control = root.querySelector<HTMLSelectElement>(`#document-option-${name}`)!
    expect(control.disabled).toBe(true)
    expect(control.parentElement!.hidden).toBe(true)
    expect(root.querySelector<HTMLLabelElement>(`label[for="${control.id}"]`)!.hidden).toBe(true)
  }
  expect(root.querySelector('output')).toBeNull()
  expect(root.querySelector('#document-page-policy')).toBeNull()
  expect(settings.metadata.fields.options).toEqual({ paper: 'a3', landscape: true, language: 'fr' })
  expect(set).not.toHaveBeenCalled()
})
it('un modèle libre retrouve les choix conservés et les menus gardent leurs liaisons', () => {
  const { root, view, set } = mount()
  view.update({ ...settings, modelId: 'free' })
  const paper = root.querySelector<HTMLSelectElement>('#document-option-paper')!
  expect(paper.parentElement!.hidden).toBe(false)
  expect(paper.disabled).toBe(false)
  expect(paper.value).toBe('a3')
  paper.value = 'a4'
  paper.dispatchEvent(new Event('input'))
  const orientation = root.querySelector<HTMLSelectElement>('#document-option-orientation')!
  orientation.value = 'false'
  orientation.dispatchEvent(new Event('input'))
  const language = root.querySelector<HTMLSelectElement>('#document-option-language')!
  language.value = 'en'
  language.dispatchEvent(new Event('input'))
  expect(set.mock.calls.map((call) => call.slice(0, 2))).toEqual([
    [{ kind: 'option', name: 'paper' }, 'a4'],
    [{ kind: 'option', name: 'landscape' }, false],
    [{ kind: 'option', name: 'language' }, 'en'],
  ])
  const model = root.querySelector<HTMLSelectElement>('#document-option-model')!
  model.value = 'personal'
  model.dispatchEvent(new Event('input'))
  expect(set).toHaveBeenLastCalledWith(
    { kind: 'model' },
    { model_id: 'personal', source: 'Typst' },
    expect.any(Function),
  )
})
it('une actualisation partagée conserve les contrôles et le focus, y compris pour un modèle embarqué rouvert', () => {
  const { root, view } = mount(),
    model = root.querySelector<HTMLSelectElement>('#document-option-model')!
  model.focus()
  view.update({ ...settings, modelId: 'embedded' })
  expect(document.activeElement).toBe(model)
  expect(model.selectedOptions[0].text).toBe('Modèle embarqué')
  expect(root.querySelector('input[type=file]')).toBeNull()
  view.update(settings)
  expect(document.activeElement).toBe(model)
  expect([...model.options].some((option) => option.value === 'embedded')).toBe(false)
})

it('une ancienne copie du modèle reste embarquée sans emprunter les contraintes du catalogue actuel', () => {
  const { root, view, set } = mount()
  view.update({ ...settings, template: 'Previous project model' })
  const model = root.querySelector<HTMLSelectElement>('#document-option-model')!
  expect(model.value).toBe('embedded')
  expect(model.selectedOptions[0].text).toBe('Modèle embarqué')
  expect(root.querySelector<HTMLSelectElement>('#document-option-paper')!.disabled).toBe(false)
  expect(set).not.toHaveBeenCalled()
  // A deliberate choice switches to the installed model, even if its original ID is unchanged.
  model.value = 'personal'
  model.dispatchEvent(new Event('input'))
  expect(set).toHaveBeenLastCalledWith(
    { kind: 'model' },
    { model_id: 'personal', source: 'Typst' },
    expect.any(Function),
  )
})

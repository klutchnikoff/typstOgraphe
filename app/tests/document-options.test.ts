import { afterEach, expect, it, vi } from 'vitest'
import { documentOptions } from '../src/document-options'
import type { Catalogue, DocumentType } from '../src/shell'
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
    changeModel = vi.fn(),
    changeOption = vi.fn()
  document.body.append(root)
  const view = documentOptions(root, kind, catalogue, settings, changeModel, changeOption)
  return { root, view, changeModel, changeOption }
}
it('un format imposé est une valeur étiquetée, sans faux menu et sans écraser les choix conservés', () => {
  const { root, changeOption } = mount()
  const paper = root.querySelector<HTMLOutputElement>('#document-option-paper-fixed')!
  expect(paper.value).toBe('A4')
  expect(paper.hidden).toBe(false)
  expect(root.querySelector('label[for="document-option-paper-fixed"]')!.textContent).toBe('Format')
  expect(root.querySelector<HTMLOutputElement>('#document-option-orientation-fixed')!.value).toBe(
    'Portrait',
  )
  expect(root.querySelector<HTMLSelectElement>('#document-option-paper')!.disabled).toBe(true)
  expect(root.querySelector<HTMLSelectElement>('#document-option-paper')!.hidden).toBe(true)
  expect(root.querySelector<HTMLElement>('#document-page-policy')!.hidden).toBe(false)
  expect(settings.metadata.fields.options).toEqual({ paper: 'a3', landscape: true, language: 'fr' })
  expect(changeOption).not.toHaveBeenCalled()
})
it('un modèle libre retrouve les choix conservés et les menus gardent leurs liaisons', () => {
  const { root, view, changeModel, changeOption } = mount()
  view.update({ ...settings, modelId: 'free' })
  const paper = root.querySelector<HTMLSelectElement>('#document-option-paper')!
  expect(paper.hidden).toBe(false)
  expect(paper.disabled).toBe(false)
  expect(paper.value).toBe('a3')
  expect(root.querySelector<HTMLOutputElement>('#document-option-paper-fixed')!.hidden).toBe(true)
  expect(root.querySelector<HTMLElement>('#document-page-policy')!.hidden).toBe(true)
  paper.value = 'a4'
  paper.dispatchEvent(new Event('change'))
  const orientation = root.querySelector<HTMLSelectElement>('#document-option-orientation')!
  orientation.value = 'false'
  orientation.dispatchEvent(new Event('change'))
  const language = root.querySelector<HTMLSelectElement>('#document-option-language')!
  language.value = 'en'
  language.dispatchEvent(new Event('change'))
  expect(changeOption.mock.calls).toEqual([
    ['paper', 'a4'],
    ['landscape', false],
    ['language', 'en'],
  ])
  const model = root.querySelector<HTMLSelectElement>('#document-option-model')!
  model.value = 'personal'
  model.dispatchEvent(new Event('change'))
  expect(changeModel).toHaveBeenCalledExactlyOnceWith('personal')
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

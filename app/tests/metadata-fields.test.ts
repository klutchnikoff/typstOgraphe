import { afterEach, expect, it, vi } from 'vitest'
import { metadataFields, refreshMetadataFields } from '../src/metadata-fields'
import type { Field, MetadataGroup } from '../src/shell'

const definitions: Field[] = [
  {
    field: 'recipient.name',
    parameter: 'destinataire',
    label: 'Destinataire',
    group: 'recipient',
    multiline: false,
    kind: 'text',
    required_for_export: true,
    value: '',
  },
  {
    field: 'recipient.address',
    parameter: 'adresse',
    label: 'Adresse',
    group: 'recipient',
    multiline: true,
    kind: 'text',
    required_for_export: false,
    value: '',
  },
  {
    field: 'letter.date',
    parameter: 'date',
    label: 'Date',
    group: 'information',
    multiline: false,
    kind: 'date',
    required_for_export: false,
    value: '',
  },
]
const group: MetadataGroup = {
  id: 'header',
  label: 'En-tête',
  sections: [
    { id: 'recipient', label: 'Destinataire', fields: ['recipient.name', 'recipient.address'] },
    { id: 'information', label: 'Informations', fields: ['letter.date'] },
  ],
}
afterEach(() => {
  document.body.replaceChildren()
  vi.useRealTimers()
})
function mount(selected = group) {
  const root = document.createElement('div'),
    change = vi.fn()
  document.body.append(root)
  metadataFields(
    root,
    selected,
    definitions,
    { destinataire: 'Camille', adresse: 'Adresse initiale', date: '2026-10-01' },
    change,
  )
  return { root, change }
}
it('les sections ordonnées du type réutilisent les identités et les contrôles des formulaires', () => {
  const { root, change } = mount()
  expect([...root.querySelectorAll('legend')].map((legend) => legend.textContent)).toEqual([
    'Destinataire',
    'Informations',
  ])
  const input = root.querySelector<HTMLInputElement>('input[type=text]')!
  expect(root.querySelector(`label[for="${input.id}"]`)!.textContent).toBe('Destinataire')
  input.value = 'Nouveau destinataire'
  input.dispatchEvent(new Event('input', { bubbles: true }))
  expect(change).toHaveBeenCalledExactlyOnceWith('recipient.name', 'Nouveau destinataire')
  expect(root.querySelector('textarea')!.value).toBe('Adresse initiale')
  const form = mount({ id: 'recipient', label: 'Destinataire', sections: [] })
  expect(form.root.querySelector('fieldset')).toBeNull()
  expect(form.root.querySelectorAll('[data-metadata-parameter]')).toHaveLength(2)
})
it('Aujourd’hui enregistre la date locale courante une fois, sans formule dynamique', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date(2026, 9, 27, 23, 58))
  const { root, change } = mount()
  root.querySelector('button')!.click()
  expect(change).toHaveBeenCalledExactlyOnceWith('letter.date', '2026-10-27')
  vi.setSystemTime(new Date(2026, 9, 29, 12))
  expect(change).toHaveBeenCalledTimes(1)
})
it('les valeurs partagées sont actualisées sans remplacer le champ focalisé ni son curseur', () => {
  const { root, change } = mount(),
    input = root.querySelector<HTMLInputElement>('input[type=text]')!
  input.focus()
  input.setSelectionRange(2, 4, 'backward')
  refreshMetadataFields(root, {
    destinataire: 'Camille',
    adresse: 'Adresse distante',
    date: '2026-10-28',
  })
  expect(document.activeElement).toBe(input)
  expect([input.selectionStart, input.selectionEnd, input.selectionDirection]).toEqual([
    2,
    4,
    'backward',
  ])
  expect(root.querySelector('textarea')!.value).toBe('Adresse distante')
  expect(root.querySelector<HTMLInputElement>('input[type=date]')!.value).toBe('2026-10-28')
  expect(change).not.toHaveBeenCalled()
})

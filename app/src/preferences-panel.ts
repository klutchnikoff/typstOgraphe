import { actionRow, keyboardScope, navigationGroup } from './keyboard-navigation'
import { tr } from './i18n'
import type { Catalogue } from './shell'
import type { Resource, Transport } from './provider'

export interface Preferences {
  appearance: string
  font: string
  size: number
  profile: Record<string, string>
  signature: Resource | null
  recent: string[]
  personal: Record<string, string[]>
  editor_fraction: number
  spelling_enabled: boolean
}

export function preferencesPanel(
  content: HTMLElement,
  catalogue: Catalogue,
  preferences: Preferences,
  transport: Transport,
  onSave: (next: Preferences) => Promise<void>,
  failed: (error: unknown) => void,
) {
  const form = document.createElement('form')
  keyboardScope(form, { confirm: () => form.requestSubmit(), tab: true })
  const fields: HTMLElement[] = []
  const addField = (label: string, input: HTMLElement) => {
    input.setAttribute('aria-label', label)
    const row = document.createElement('label')
    row.className = 'field'
    row.append(document.createTextNode(label), input)
    fields.push(row)
  }
  const appearance = document.createElement('select')
  for (const [value, label] of [
    ['system', tr('Système')],
    ['light', tr('Clair')],
    ['dark', tr('Sombre')],
  ])
    appearance.add(new Option(label, value))
  appearance.value = preferences.appearance
  addField(tr('Apparence'), appearance)
  const font = document.createElement('select')
  for (const [value, label] of [
    ['serif', tr('Sérif')],
    ['sans', tr('Sans empattement')],
    ['mono', tr('Monospace')],
  ])
    font.add(new Option(label, value))
  font.value = preferences.font
  addField(tr('Police de l’éditeur'), font)
  const size = document.createElement('input')
  size.type = 'number'
  size.min = '12'
  size.max = '32'
  size.value = String(preferences.size)
  addField(tr('Taille'), size)
  const spelling = document.createElement('input')
  spelling.type = 'checkbox'
  spelling.checked = preferences.spelling_enabled !== false
  addField(tr('Correction orthographique'), spelling)
  const personal = structuredClone(preferences.personal || {})
  const dictionary = document.createElement('div')
  navigationGroup(dictionary, { axis: 'vertical' })
  fields.push(dictionary)
  for (const [language, words] of Object.entries(personal)) {
    for (const word of words) {
      const row = document.createElement('div')
      const label = document.createElement('span')
      label.textContent = `${language} · ${word}`
      const remove = document.createElement('button')
      remove.type = 'button'
      remove.textContent = tr('Supprimer')
      remove.setAttribute('aria-label', `${tr('Supprimer')} ${word}`)
      remove.onclick = () => {
        personal[language] = personal[language].filter((w) => w !== word)
        row.remove()
      }
      row.append(label, remove)
      dictionary.append(row)
    }
  }
  const profileInputs = new Map<string, HTMLInputElement | HTMLTextAreaElement>()
  for (const definition of catalogue.types[0].fields.filter((f) => f.group === 'sender')) {
    const input = definition.multiline
      ? document.createElement('textarea')
      : document.createElement('input')
    input.value = preferences.profile[definition.field] || ''
    profileInputs.set(definition.field, input)
    addField(definition.label, input)
  }
  const signature = document.createElement('input')
  signature.type = 'file'
  signature.accept = 'image/png,image/jpeg,image/svg+xml'
  addField(tr('Signature par défaut'), signature)
  let currentSignature = preferences.signature
  if (currentSignature) {
    const preview = document.createElement('img')
    const extension = currentSignature.path.split('.').pop()
    preview.src = `data:image/${extension === 'svg' ? 'svg+xml' : extension === 'jpg' ? 'jpeg' : extension};base64,${currentSignature.data}`
    preview.alt = tr('Signature par défaut')
    preview.style.maxWidth = '160px'
    const remove = document.createElement('button')
    remove.type = 'button'
    remove.textContent = tr('Supprimer la signature')
    remove.onclick = () => {
      currentSignature = null
      preview.remove()
      remove.remove()
    }
    fields.push(preview, remove)
  }
  const save = document.createElement('button')
  save.type = 'submit'
  save.className = 'primary'
  save.textContent = tr('Enregistrer les préférences')
  const actions = actionRow()
  actions.append(save)
  form.append(...fields, actions)
  form.onsubmit = (e) => {
    e.preventDefault()
    void (async () => {
      const next: Preferences = {
        ...preferences,
        appearance: appearance.value,
        font: font.value,
        size: Number(size.value),
        spelling_enabled: spelling.checked,
        personal,
        signature: currentSignature,
        profile: Object.fromEntries([...profileInputs].map(([key, input]) => [key, input.value])),
      }
      const file = signature.files?.[0]
      if (file) {
        next.signature = await transport.invoke<Resource>('profile_image', {
          extension: file.name.split('.').pop(),
          data: Array.from(new Uint8Array(await file.arrayBuffer())),
        })
      }
      await onSave(next)
    })().catch(failed)
  }
  content.append(form)
}

import { tr } from './i18n'
import { icon } from './icons'
import type { Field, MetadataGroup } from './shell'
import type { MetadataCommands } from './metadata-commands'
import { bindMetadataInput, metadataInputBinding } from './metadata-input'

type Section = MetadataGroup['sections'][number]
type ChangeField = (field: string, value: string) => void

/** Shared section frame for type-defined fields and specialised metadata controls. */
export function metadataSection(section: Pick<Section, 'id' | 'label' | 'layout'>) {
  const inline = section.layout === 'inline'
  const root = document.createElement(inline ? 'div' : 'fieldset')
  root.className = `metadata-section metadata-section-${section.layout || 'editorial'}`
  root.dataset.section = section.id
  if (!inline) {
    const legend = document.createElement('legend')
    legend.textContent = section.label
    root.append(legend)
  }
  const content = document.createElement('div')
  content.className = 'metadata-section-fields'
  root.append(content)
  return { root, content }
}

/** Every type uses the same control and auxiliary actions for each field kind. */
function metadataControl(
  definition: Field,
  value: string,
  change: ChangeField,
  editInput: ChangeField = change,
) {
  const controls = document.createElement('div')
  controls.className = 'metadata-field-controls'
  const input = definition.multiline
    ? document.createElement('textarea')
    : document.createElement('input')
  input.id = `metadata-field-${definition.field}`
  input.dataset.metadataParameter = definition.parameter
  if (input instanceof HTMLInputElement) input.type = definition.kind === 'date' ? 'date' : 'text'
  if (input instanceof HTMLTextAreaElement) input.rows = 3
  input.value = value
  input.oninput = () => {
    refreshMetadataActions(input)
    editInput(definition.field, input.value)
  }
  controls.append(input)
  if (definition.kind === 'date') {
    controls.classList.add('metadata-date-controls')
    const today = document.createElement('button')
    today.type = 'button'
    today.textContent = tr('Aujourd’hui')
    today.onclick = () => {
      const now = new Date()
      const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
      change(definition.field, date)
    }
    controls.append(today)
    if (!definition.required_for_export) {
      const clear = document.createElement('button')
      clear.type = 'button'
      clear.className = 'metadata-date-clear'
      clear.title = tr('Supprimer la date')
      clear.setAttribute('aria-label', clear.title)
      clear.innerHTML = icon('trash-2')
      clear.querySelector('svg')?.setAttribute('aria-hidden', 'true')
      clear.disabled = input.value === ''
      clear.onclick = () => {
        change(definition.field, '')
        input.focus()
      }
      controls.append(clear)
    }
  }
  return { controls, input }
}

/** Accessible label and control row shared by compact and editorial layouts. */
function metadataField(
  definition: Field,
  value: string,
  change: ChangeField,
  editInput: ChangeField = change,
) {
  const row = document.createElement('div')
  row.className = 'field'
  const label = document.createElement('label')
  label.textContent = definition.label
  const { controls, input } = metadataControl(definition, value, change, editInput)
  label.htmlFor = input.id
  row.append(label, controls)
  return row
}

/** The type describes grouping and order; rendering never branches on the document type. */
export function metadataFields(
  parent: HTMLElement,
  group: MetadataGroup,
  definitions: Field[],
  values: Record<string, unknown>,
  change: ChangeField,
  binding?: { commands: MetadataCommands; read: (field: Field) => unknown },
) {
  const append = (container: HTMLElement, definition: Field) => {
    const row = metadataField(
      definition,
      String(values[definition.parameter] ?? ''),
      change,
      binding ? () => {} : change,
    )
    container.append(row)
    if (binding)
      bindMetadataInput(
        row.querySelector('input, textarea')!,
        binding.commands,
        { kind: 'metadata', field: definition.field },
        () => binding.read(definition),
      )
  }
  if (!group.sections.length) {
    for (const definition of definitions.filter((field) => field.group === group.id))
      append(parent, definition)
    return
  }
  for (const section of group.sections) {
    const { root, content } = metadataSection(section)
    for (const id of section.fields) {
      const definition = definitions.find((field) => field.field === id)
      if (definition) append(content, definition)
    }
    parent.append(root)
  }
}

function refreshMetadataActions(input: HTMLInputElement | HTMLTextAreaElement) {
  const clear = input.parentElement?.querySelector<HTMLButtonElement>('.metadata-date-clear')
  if (clear) clear.disabled = input.value === ''
}

/** Refresh shared values without rebuilding the control being edited. */
export function refreshMetadataFields(parent: HTMLElement, values: Record<string, unknown>) {
  for (const input of parent.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    '[data-metadata-parameter]',
  )) {
    const binding = metadataInputBinding(input)
    if (binding) {
      binding.refresh()
      refreshMetadataActions(input)
      continue
    }
    const value = String(values[input.dataset.metadataParameter!] ?? '')
    if (input.value === value) {
      refreshMetadataActions(input)
      continue
    }
    const start = input.selectionStart,
      end = input.selectionEnd,
      direction = input.selectionDirection
    input.value = value
    refreshMetadataActions(input)
    if (document.activeElement === input && start !== null && end !== null)
      input.setSelectionRange(
        Math.min(start, value.length),
        Math.min(end, value.length),
        direction || undefined,
      )
  }
}
export function destroyMetadataFields(parent: HTMLElement) {
  for (const input of parent.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    '[data-metadata-parameter]',
  ))
    metadataInputBinding(input)?.destroy()
}

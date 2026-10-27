import { tr } from './i18n'
import type { Field, MetadataGroup } from './shell'

/** Native controls shared by form tabs and the type's editorial sections. */
export function metadataFields(
  parent: HTMLElement,
  group: MetadataGroup,
  definitions: Field[],
  values: Record<string, unknown>,
  change: (field: string, value: string) => void,
) {
  const append = (container: HTMLElement, definition: Field) => {
    const row = document.createElement('div')
    row.className = 'field'
    const label = document.createElement('label')
    label.textContent = definition.label
    const input = definition.multiline
      ? document.createElement('textarea')
      : document.createElement('input')
    input.id = `metadata-field-${definition.field}`
    label.htmlFor = input.id
    input.dataset.metadataParameter = definition.parameter
    if (input instanceof HTMLInputElement) input.type = definition.kind === 'date' ? 'date' : 'text'
    if (input instanceof HTMLTextAreaElement) input.rows = 3
    input.value = String(values[definition.parameter] ?? '')
    input.oninput = () => change(definition.field, input.value)
    const controls = document.createElement('div')
    controls.className = 'metadata-field-controls'
    controls.append(input)
    if (definition.kind === 'date') {
      const today = document.createElement('button')
      today.type = 'button'
      today.textContent = tr('Aujourd’hui')
      today.onclick = () => {
        const now = new Date()
        const value = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
        change(definition.field, value)
      }
      controls.append(today)
    }
    row.append(label, controls)
    container.append(row)
  }
  if (!group.sections.length) {
    for (const definition of definitions.filter((field) => field.group === group.id))
      append(parent, definition)
    return
  }
  for (const section of group.sections) {
    const fieldset = document.createElement('fieldset')
    fieldset.className = 'metadata-section'
    fieldset.dataset.section = section.id
    const legend = document.createElement('legend')
    legend.textContent = section.label
    const fields = document.createElement('div')
    fields.className = 'metadata-section-fields'
    for (const id of section.fields) {
      const definition = definitions.find((field) => field.field === id)
      if (definition) append(fields, definition)
    }
    fieldset.append(legend, fields)
    parent.append(fieldset)
  }
}

/** Refresh shared values without rebuilding the control being edited. */
export function refreshMetadataFields(parent: HTMLElement, values: Record<string, unknown>) {
  for (const input of parent.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    '[data-metadata-parameter]',
  )) {
    const value = String(values[input.dataset.metadataParameter!] ?? '')
    if (input.value === value) continue
    const start = input.selectionStart,
      end = input.selectionEnd,
      direction = input.selectionDirection
    input.value = value
    if (document.activeElement === input && start !== null && end !== null)
      input.setSelectionRange(
        Math.min(start, value.length),
        Math.min(end, value.length),
        direction || undefined,
      )
  }
}

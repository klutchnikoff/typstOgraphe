import { labeledField } from './form-field'
import type { Field, MetadataGroup } from './shell'
import type { MetadataCommands } from './metadata-commands'
import { bindMetadataInput, metadataInputBinding } from './metadata-input'
import { metadataDate, metadataDateControl } from './metadata-date'
import {
  metadataResource,
  metadataResourceControl,
  type ResourceContext,
} from './metadata-resource'

type Section = MetadataGroup['sections'][number]
type ChangeField = (field: string, value: unknown) => void

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
  if (input instanceof HTMLInputElement) input.type = 'text'
  if (input instanceof HTMLTextAreaElement) input.rows = 3
  input.value = value
  input.oninput = () => {
    editInput(definition.field, input.value)
  }
  controls.append(input)
  return { controls, input }
}

/** Accessible label and control row shared by compact and editorial layouts. */
function metadataField(
  definition: Field,
  value: unknown,
  change: ChangeField,
  editInput: ChangeField = change,
  binding?: {
    commands: MetadataCommands
    read: (field: Field) => unknown
    resources?: ResourceContext
  },
) {
  const date = definition.resource
    ? metadataResource(definition, value, change, binding)
    : definition.kind === 'date'
      ? metadataDate(definition, value, change, binding)
      : undefined
  const { controls, input } =
    date ?? metadataControl(definition, String(value ?? ''), change, editInput)
  const row = labeledField(definition.label, input, controls)
  if (definition.resource) row.classList.add('metadata-resource-field')
  date?.bind()
  return row
}

/** The type describes grouping and order; rendering never branches on the document type. */
export function metadataFields(
  parent: HTMLElement,
  group: MetadataGroup,
  definitions: Field[],
  values: Record<string, unknown>,
  change: ChangeField,
  binding?: {
    commands: MetadataCommands
    read: (field: Field) => unknown
    resources?: ResourceContext
  },
) {
  const append = (container: HTMLElement, definition: Field, hideLabel = false) => {
    const row = metadataField(
      definition,
      values[definition.parameter],
      change,
      binding ? () => {} : change,
      binding,
    )
    if (hideLabel) row.querySelector('label')!.classList.add('sr-only')
    container.append(row)
    if (binding && definition.kind !== 'date' && !definition.resource)
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
      if (definition)
        append(
          content,
          definition,
          !!definition.resource &&
            section.fields.length === 1 &&
            section.label === definition.label,
        )
    }
    parent.append(root)
  }
}

/** Refresh shared values without rebuilding the control being edited. */
export function refreshMetadataFields(parent: HTMLElement, values: Record<string, unknown>) {
  for (const root of parent.querySelectorAll<HTMLElement>('[data-metadata-resource]'))
    metadataResourceControl(root)?.refresh(values[root.dataset.metadataResource!])
  for (const root of parent.querySelectorAll<HTMLElement>('[data-metadata-date]'))
    metadataDateControl(root)?.refresh(values[root.dataset.metadataDate!])
  for (const input of parent.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    '[data-metadata-parameter]',
  )) {
    const binding = metadataInputBinding(input)
    if (binding) {
      binding.refresh()
      continue
    }
    const value = String(values[input.dataset.metadataParameter!] ?? '')
    if (input.value === value) {
      continue
    }
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
export function destroyMetadataFields(parent: HTMLElement) {
  for (const root of parent.querySelectorAll<HTMLElement>('[data-metadata-resource]'))
    metadataResourceControl(root)?.destroy()
  for (const root of parent.querySelectorAll<HTMLElement>('[data-metadata-date]'))
    metadataDateControl(root)?.destroy()
  for (const input of parent.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    '[data-metadata-parameter]',
  ))
    metadataInputBinding(input)?.destroy()
}

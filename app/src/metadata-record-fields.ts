import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import type { Catalogue } from './shell'
import { historyFor } from './editing-history'
import { metadataCommands } from './metadata-commands'
import { metadataRich } from './metadata-rich'
import { richFieldProfile } from './rich-zone'
import { bindMetadataInput } from './metadata-input'
import { tr } from './i18n'
import {
  recordIds,
  recordMap,
  setRecordField,
  type StructuredMetadata,
  type RecordCollection,
  type RecordStore,
  type SharedRecord,
  type RecordField,
} from './metadata-records'
import * as Y from 'yjs'

/** Field rendering and bindings shared by live records and isolated modal drafts.
 * The collection controller owns ordering/dialog lifetime; this renderer only owns
 * its controls, rich zones and their subscriptions.
 */
export function createRecordFields(
  provider: Provider,
  schema: StructuredMetadata,
  catalogue: Catalogue,
  main: Editor,
  language: () => string,
) {
  const commands = metadataCommands(main, provider)
  const history = historyFor(main)
  const definitions = new Map(schema.collections.map((item) => [item.parameter, item]))
  const discrete = (action: () => void) => {
    history?.stopCapturing()
    try {
      action()
    } finally {
      history?.stopCapturing()
    }
  }
  function fieldView(
    container: HTMLElement,
    definition: RecordCollection,
    record: SharedRecord,
    field: RecordField,
    store: RecordStore = provider,
    owner: Editor = main,
  ) {
    const row = document.createElement('div')
    row.className = 'field metadata-record-field'
    row.dataset.recordField = field.key
    row.dataset.fieldKind = field.kind
    row.classList.toggle(
      'metadata-record-wide',
      field.key === definition.title_key || field.kind !== 'text',
    )
    container.append(row)
    if (field.kind === 'rich' || field.kind === 'rich_inline') {
      const rich = metadataRich(
        row,
        field.label,
        record.get(field.key) as Y.XmlFragment,
        richFieldProfile(field.kind),
        owner,
        catalogue,
        String(record.get(definition.language_key || '') || language()),
        {
          kind: 'record',
          collection: definition.parameter,
          id: String(record.get('id')),
          field: field.key,
        },
        field.statistics ? provider.transport : undefined,
      )
      return {
        refresh: () =>
          rich.updateLanguage(String(record.get(definition.language_key || '') || language())),
        destroy: rich.destroy,
      }
    }
    const label = document.createElement('label')
    label.textContent = field.label
    const id = `record-field-${definition.parameter}-${record.get('id')}-${field.key}`
    label.htmlFor = id
    label.id = `${id}-label`
    if (field.kind === 'references') {
      const choices = document.createElement('div')
      // Keep removed choices until this form closes, even after unchecking:
      // removing a focused checkbox during its change event loses keyboard focus.
      const missing = new Set<string>()
      choices.className = 'metadata-reference-choices'
      choices.setAttribute('role', 'group')
      choices.setAttribute('aria-labelledby', label.id)
      row.append(label, choices)
      const publicTarget = {
        kind: 'record' as const,
        collection: definition.parameter,
        id: String(record.get('id')),
        field: field.key,
      }
      const selectedValues = () =>
        (store === provider
          ? commands.value(publicTarget, record.get(field.key))
          : record.get(field.key)) as string[]
      let stop = () => {}
      const view = {
        destroy: () => stop(),
        refresh() {
          const target = definitions.get(field.references!)!
          const ids = recordIds(provider, target)
          const selected = selectedValues()
          for (const key of selected) if (!ids.includes(key)) missing.add(key)
          for (const key of ids) missing.delete(key)
          for (const child of [...choices.children])
            if (
              ![...ids, ...selected, ...missing].includes((child as HTMLElement).dataset.reference!)
            )
              child.remove()
          for (const key of new Set([...ids, ...selected, ...missing])) {
            let choice = [...choices.children].find(
              (child) => (child as HTMLElement).dataset.reference === key,
            ) as HTMLLabelElement | undefined
            if (!choice) {
              choice = document.createElement('label')
              choice.dataset.reference = key
              const checkbox = document.createElement('input')
              checkbox.type = 'checkbox'
              const caption = document.createElement('span')
              choice.append(checkbox, caption)
              choices.append(choice)
              checkbox.onchange = () => {
                const selected = new Set(selectedValues())
                if (checkbox.checked) selected.add(key)
                else selected.delete(key)
                if (store === provider)
                  commands.set(
                    {
                      kind: 'record',
                      collection: definition.parameter,
                      id: String(record.get('id')),
                      field: field.key,
                    },
                    [...selected],
                    () => record.get(field.key),
                  )
                else discrete(() => setRecordField(store, record, field.key, [...selected]))
              }
            }
            const reference = recordMap(provider, target).get(key)
            choice.querySelector('span')!.textContent = reference
              ? String(reference.get(target.title_key) || '') || target.itemLabel
              : `${target.itemLabel} · ${tr('Élément supprimé')}`
            choice.querySelector('input')!.checked = selectedValues().includes(key)
          }
        },
      }
      if (store === provider) stop = commands.watch(() => view.refresh())
      return view
    }
    const input =
      field.kind === 'multiline' || field.kind === 'tokens'
        ? document.createElement('textarea')
        : field.kind === 'language' || field.kind === 'choice'
          ? document.createElement('select')
          : document.createElement('input')
    input.id = id
    if (input instanceof HTMLTextAreaElement) input.rows = field.kind === 'tokens' ? 2 : 3
    if (input instanceof HTMLInputElement)
      input.type = field.kind === 'boolean' ? 'checkbox' : 'text'
    if (input instanceof HTMLSelectElement) {
      const choices =
        field.kind === 'choice'
          ? (field.choices || []).map((value) => ({ label: value, value }))
          : catalogue.languages
      for (const item of choices) input.add(new Option(item.label, item.value))
    }
    input.oninput = () => {
      const value =
        field.kind === 'boolean'
          ? (input as HTMLInputElement).checked
          : field.kind === 'tokens'
            ? input.value.split('\n').filter((s) => s.trim() !== '')
            : input.value
      if (store === provider) return // The common native-field binding owns live publication and IME.
      const update = () => setRecordField(store, record, field.key, value)
      if (field.kind === 'boolean' || field.kind === 'language' || field.kind === 'choice')
        discrete(update)
      else update()
    }
    input.onfocus = input.onblur = () => history?.stopCapturing()
    if (field.kind === 'tokens')
      input.setAttribute(
        'aria-description',
        tr('Un mot-clé par ligne ; les expressions peuvent contenir des espaces.'),
      )
    if (field.kind === 'boolean') {
      label.prepend(input)
      row.append(label)
    } else row.append(label, input)
    input.dataset.fieldKind = field.kind
    const binding =
      store === provider
        ? bindMetadataInput(
            input,
            commands,
            {
              kind: 'record',
              collection: definition.parameter,
              id: String(record.get('id')),
              field: field.key,
            },
            () => record.get(field.key),
            () =>
              field.kind === 'boolean'
                ? (input as HTMLInputElement).checked
                : field.kind === 'tokens'
                  ? input.value.split('\n').filter((value) => value.trim() !== '')
                  : input.value,
            (value) =>
              field.kind === 'tokens'
                ? ((value as string[]) || []).join('\n')
                : String(value ?? ''),
          )
        : undefined
    return {
      destroy: () => binding?.destroy(),
      refresh() {
        if (binding) binding.refresh()
        else if (field.kind === 'boolean')
          (input as HTMLInputElement).checked = Boolean(record.get(field.key))
        else {
          const current =
            field.kind === 'tokens'
              ? (record.get(field.key) as string[]).join('\n')
              : String(record.get(field.key) || '')
          if (
            input.value !== current &&
            !(field.kind === 'tokens' && document.activeElement === input)
          ) {
            const text =
              input instanceof HTMLInputElement || input instanceof HTMLTextAreaElement
                ? input
                : undefined
            const start = text?.selectionStart,
              end = text?.selectionEnd
            input.value = current
            if (document.activeElement === text && start != null && end != null)
              text!.setSelectionRange(
                Math.min(start, current.length),
                Math.min(end, current.length),
              )
          }
        }
        if (input instanceof HTMLSelectElement) {
          const used = recordIds(provider, definition)
            .filter((key) => key !== record.get('id'))
            .map((key) => recordMap(provider, definition).get(key)!.get(field.key))
          for (const option of input.options) option.disabled = used.includes(option.value)
        }
      },
    }
  }
  return fieldView
}

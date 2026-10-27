import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import type { Catalogue } from './shell'
import { metadataSection } from './metadata-fields'
import { metadataRich } from './metadata-rich'
import { contextPanel } from './context-panel'
import { actionRow, navigationGroup } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import {
  addRecord,
  setRecordField,
  recordMap,
  recordIds,
  moveRecord,
  removeRecord,
  type StructuredMetadata,
  type RecordCollection,
  type SharedRecord,
  type RecordField,
} from './metadata-records'
import * as Y from 'yjs'
import { yUndoPluginKey } from '@tiptap/y-tiptap'

type View = { record: SharedRecord; element: HTMLElement; refresh: () => void; destroy: () => void }

/** Type-driven repeatable records. No document-type names or publication layout rules here. */
export function metadataCollections(
  parent: HTMLElement,
  provider: Provider,
  schema: StructuredMetadata,
  catalogue: Catalogue,
  main: Editor,
  language: () => string,
) {
  const history = yUndoPluginKey.getState(main.state)?.undoManager
  const discrete = (action: () => void) => {
    history?.stopCapturing()
    try {
      action()
    } finally {
      history?.stopCapturing()
    }
  }
  const root = document.createElement('div')
  root.className = 'metadata-collections'
  parent.append(root)
  const primaryRoot = document.createElement('div')
  primaryRoot.className = 'metadata-primary-fields'
  root.append(primaryRoot)
  let primaryId = '',
    primaryRecord: SharedRecord | undefined
  let primaryViews: ReturnType<typeof fieldView>[] = []
  const collections = new Map<
    RecordCollection,
    { root: HTMLElement; views: Map<string, View>; add: HTMLButtonElement }
  >()
  const definitions = new Map(
    schema.collections.map((definition) => [definition.parameter, definition]),
  )
  const button = (label: string, glyph: string, action: () => void) => {
    const element = document.createElement('button')
    element.type = 'button'
    element.title = label
    element.setAttribute('aria-label', label)
    element.innerHTML = icon(glyph)
    element.onclick = (event) => {
      event.preventDefault()
      event.stopPropagation()
      discrete(action)
    }
    return element
  }
  function fieldView(
    container: HTMLElement,
    definition: RecordCollection,
    record: SharedRecord,
    field: RecordField,
  ) {
    const row = document.createElement('div')
    row.className = 'field metadata-record-field'
    row.dataset.recordField = field.key
    container.append(row)
    if (field.kind === 'rich' || field.kind === 'rich_inline') {
      const rich = metadataRich(
        row,
        field.label,
        record.get(field.key) as Y.XmlFragment,
        field.kind === 'rich_inline',
        main,
        catalogue,
        String(record.get(definition.language_key || '') || language()),
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
    if (field.kind === 'references') {
      const choices = document.createElement('div')
      choices.className = 'metadata-reference-choices'
      row.append(label, choices)
      return {
        destroy: () => {},
        refresh() {
          const target = definitions.get(field.references!)!
          const ids = recordIds(provider, target)
          for (const child of [...choices.children])
            if (!ids.includes((child as HTMLElement).dataset.reference!)) child.remove()
          for (const key of ids) {
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
                const selected = new Set(record.get(field.key) as string[])
                if (checkbox.checked) selected.add(key)
                else selected.delete(key)
                discrete(() => setRecordField(provider, record, field.key, [...selected]))
              }
            }
            choice.querySelector('span')!.textContent =
              String(recordMap(provider, target).get(key)!.get(target.title_key) || '') ||
              target.itemLabel
            choice.querySelector('input')!.checked = (record.get(field.key) as string[]).includes(
              key,
            )
          }
        },
      }
    }
    const input =
      field.kind === 'multiline' || field.kind === 'tokens'
        ? document.createElement('textarea')
        : field.kind === 'language'
          ? document.createElement('select')
          : document.createElement('input')
    input.id = id
    if (input instanceof HTMLTextAreaElement) input.rows = field.kind === 'tokens' ? 2 : 3
    if (input instanceof HTMLInputElement)
      input.type = field.kind === 'boolean' ? 'checkbox' : 'text'
    if (input instanceof HTMLSelectElement)
      for (const item of catalogue.languages) input.add(new Option(item.label, item.value))
    input.oninput = () => {
      const value =
        field.kind === 'boolean'
          ? (input as HTMLInputElement).checked
          : field.kind === 'tokens'
            ? input.value.split('\n').filter((s) => s.trim() !== '')
            : input.value
      const update = () => setRecordField(provider, record, field.key, value)
      if (field.kind === 'boolean' || field.kind === 'language') discrete(update)
      else update()
    }
    input.onfocus = input.onblur = () => history?.stopCapturing()
    if (field.kind === 'tokens')
      input.setAttribute(
        'aria-description',
        tr('Un mot-clé par ligne ; les expressions peuvent contenir des espaces.'),
      )
    row.append(label, input)
    return {
      destroy: () => {},
      refresh() {
        if (field.kind === 'boolean')
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
  function recordView(definition: RecordCollection, record: SharedRecord, id: string): View {
    const element = document.createElement('details')
    element.className = 'metadata-record'
    element.dataset.recordId = id
    const summary = document.createElement('summary')
    const caption = document.createElement('span')
    caption.className = 'metadata-record-caption'
    const actions = actionRow()
    const fieldsRoot = document.createElement('div')
    fieldsRoot.className = 'metadata-record-fields'
    const deletion = document.createElement('div')
    deletion.className = 'metadata-record-deletion'
    const confirm = contextPanel({
      id: `delete-record-${definition.parameter}-${id}`,
      closeLabel: tr('Annuler'),
      close: () => {
        confirm.element.hidden = true
        remove.focus()
      },
      canConfirm: () => true,
    })
    confirm.title.textContent = `${tr('Supprimer')} · ${definition.itemLabel}`
    const detach = document.createElement('label'),
      check = document.createElement('input')
    check.type = 'checkbox'
    detach.append(
      check,
      document.createTextNode(tr('Retirer aussi les associations à cette affiliation.')),
    )
    confirm.body.append(detach)
    confirm.action({
      label: tr('Annuler'),
      run: () => {
        confirm.element.hidden = true
        remove.focus()
      },
    })
    confirm.action({
      label: tr('Supprimer'),
      danger: true,
      primary: true,
      run: () => {
        if (definition.language_key && record.get(definition.language_key) === language()) {
          confirm.error.textContent = tr('L’ensemble de la langue principale doit être conservé.')
          return
        }
        let removed = false
        discrete(() => {
          removed = removeRecord(provider, schema, definition, id, check.checked)
        })
        if (!removed) {
          detach.hidden = false
          confirm.error.textContent = tr(
            'Cette affiliation est utilisée. Confirmez le retrait de ses associations.',
          )
          return
        }
        collections.get(definition)!.add.focus()
      },
    })
    deletion.append(confirm.element)
    const remove = button(`${tr('Supprimer')} · ${definition.itemLabel}`, 'trash-2', () => {
      confirm.error.textContent = ''
      check.checked = false
      detach.hidden = true
      element.open = true
      confirm.element.hidden = false
      confirm.focus()
    })
    const up = button(`${tr('Monter')} · ${definition.itemLabel}`, 'arrow-up', () =>
      moveRecord(provider, definition, id, -1),
    )
    const down = button(`${tr('Descendre')} · ${definition.itemLabel}`, 'arrow-down', () =>
      moveRecord(provider, definition, id, 1),
    )
    const edit = button(`${tr('Modifier')} · ${definition.itemLabel}`, 'settings-2', () => {
      element.open = !element.open
      if (element.open)
        fieldsRoot
          .querySelector<HTMLElement>('input, textarea, select, [contenteditable=true]')
          ?.focus()
    })
    actions.append(remove, up, down, edit)
    summary.append(caption, actions)
    element.append(summary, deletion, fieldsRoot)
    collections.get(definition)!.root.append(element)
    let views: ReturnType<typeof fieldView>[] = [],
      mode = ''
    const render = () => {
      const isPrimary =
        !!definition.language_key && record.get(definition.language_key) === language()
      const nextMode = isPrimary ? 'primary' : 'other'
      if (mode !== nextMode) {
        for (const view of views) view.destroy()
        fieldsRoot.replaceChildren()
        mode = nextMode
        views = definition.fields
          .filter(
            (field) =>
              !isPrimary ||
              (!definition.primary_fields?.includes(field.key) && field.kind !== 'language'),
          )
          .map((field) => fieldView(fieldsRoot, definition, record, field))
      }
      const value = String(record.get(definition.title_key) ?? '')
      caption.textContent = definition.language_key
        ? catalogue.languages.find((item) => item.value === value)?.label || definition.itemLabel
        : String(value || definition.itemLabel)
      const ids = recordIds(provider, definition),
        index = ids.indexOf(id)
      up.disabled = index === 0
      down.disabled = index === ids.length - 1
      remove.disabled = isPrimary
      for (const view of views) view.refresh()
    }
    element.open = !!definition.language_key && record.get(definition.language_key) === language()
    render()
    return {
      record,
      element,
      refresh: render,
      destroy() {
        for (const view of views) view.destroy()
        element.remove()
      },
    }
  }
  for (const definition of schema.collections) {
    const section = metadataSection({
      id: definition.parameter,
      label: definition.label,
      layout: 'editorial',
    })
    section.root.classList.add('metadata-record-section')
    const add = document.createElement('button')
    add.type = 'button'
    add.textContent = `${tr('Ajouter')} · ${definition.itemLabel}`
    const actions = actionRow()
    actions.append(add)
    section.content.append(actions)
    root.append(section.root)
    collections.set(definition, { root: section.content, views: new Map(), add })
    add.onclick = () => {
      const langKey = definition.language_key
      const used = langKey
        ? recordIds(provider, definition).map((id) =>
            recordMap(provider, definition).get(id)!.get(langKey),
          )
        : []
      const next = catalogue.languages.find((item) => !used.includes(item.value))?.value
      if (langKey && !next) return
      history?.stopCapturing()
      const record = addRecord(provider, definition, langKey ? { [langKey]: next } : {})
      history?.stopCapturing()
      refresh()
      const view = collections.get(definition)!.views.get(record.get('id') as string)
      if (view) {
        ;(view.element as HTMLDetailsElement).open = true
        view.element
          .querySelector<HTMLElement>(
            '.metadata-record-fields input, .metadata-record-fields textarea, .metadata-record-fields select, [contenteditable=true]',
          )
          ?.focus()
      }
    }
  }
  function refresh() {
    const localized = schema.collections.find((definition) => definition.language_key)
    const current =
      localized &&
      recordIds(provider, localized)
        .map((id) => recordMap(provider, localized).get(id)!)
        .find((record) => record.get(localized.language_key!) === language())
    if (current !== primaryRecord || (current?.get('id') || '') !== primaryId) {
      for (const view of primaryViews) view.destroy()
      primaryRoot.replaceChildren()
      primaryRecord = current
      primaryId = String(current?.get('id') || '')
      primaryViews =
        current && localized
          ? (localized.primary_fields || [])
              .map((key) => localized.fields.find((field) => field.key === key)!)
              .map((field) => fieldView(primaryRoot, localized, current, field))
          : []
    }
    for (const view of primaryViews) view.refresh()
    for (const [definition, state] of collections) {
      const ids = recordIds(provider, definition)
      for (const [id, view] of state.views)
        if (!ids.includes(id)) {
          view.destroy()
          state.views.delete(id)
        }
      let previous: ChildNode = state.add.parentElement!
      const focused = document.activeElement as HTMLElement | null
      for (const id of ids) {
        let view = state.views.get(id)
        if (view && view.record !== recordMap(provider, definition).get(id)) {
          view.destroy()
          state.views.delete(id)
          view = undefined
        }
        if (!view) {
          view = recordView(definition, recordMap(provider, definition).get(id)!, id)
          state.views.set(id, view)
        }
        view.refresh()
        if (previous.nextSibling !== view.element)
          state.root.insertBefore(view.element, previous.nextSibling)
        previous = view.element
      }
      if (focused?.isConnected && root.contains(focused) && document.activeElement !== focused)
        focused.focus({ preventScroll: true })
      state.add.disabled = !!definition.language_key && ids.length >= catalogue.languages.length
    }
  }
  refresh()
  return {
    refresh,
    destroy() {
      for (const view of primaryViews) view.destroy()
      for (const state of collections.values())
        for (const view of state.views.values()) view.destroy()
      root.remove()
    },
  }
}

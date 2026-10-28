import { historyFor } from './editing-history'
import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import type { Catalogue } from './shell'
import { metadataSection } from './metadata-fields'
import { metadataRich } from './metadata-rich'
import { createRichZone, richFieldProfile } from './rich-zone'
import { contextPanel } from './context-panel'
import { formDialog } from './form-dialog'
import { actionRow } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import { metadataCommands } from './metadata-commands'
import { bindMetadataInput } from './metadata-input'
import { moveRecordCommands, removeRecordCommands } from './metadata-record-commands'
import {
  prepareRecordEdit,
  setRecordField,
  recordMap,
  recordIds,
  type StructuredMetadata,
  type MetadataPanel,
  type RecordStore,
  type RecordCollection,
  type SharedRecord,
  type RecordField,
} from './metadata-records'
import * as Y from 'yjs'

type View = { record: SharedRecord; element: HTMLElement; refresh: () => void; destroy: () => void }

/** Type-driven repeatable records. No document-type names or publication layout rules here. */
export function metadataCollections(
  parent: HTMLElement,
  provider: Provider,
  schema: StructuredMetadata,
  panel: MetadataPanel,
  catalogue: Catalogue,
  main: Editor,
  language: () => string,
) {
  const history = historyFor(main)
  const commands = metadataCommands(main, provider)
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
  const collections = new Map<
    RecordCollection,
    {
      root: HTMLElement
      views: Map<string, View>
      add: HTMLButtonElement
      draft?: {
        element: HTMLElement
        floating: boolean
        refresh: () => void
        destroy: () => void
        focus: () => void
      }
    }
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
      if (store === provider) return // The common native-field binding owns live publication and IME.
      const update = () => setRecordField(store, record, field.key, value)
      if (field.kind === 'boolean' || field.kind === 'language') discrete(update)
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
  function recordView(definition: RecordCollection, record: SharedRecord, id: string): View {
    const element = document.createElement('details')
    element.className = 'metadata-record'
    element.dataset.recordId = id
    const summary = document.createElement('summary')
    const caption = document.createElement('span')
    caption.className = 'metadata-record-caption'
    const label = document.createElement('span')
    label.className = 'metadata-record-label'
    const detail = document.createElement('small')
    detail.className = 'metadata-record-detail'
    label.append(caption, detail)
    const actions = actionRow()
    const fieldsRoot = document.createElement('div')
    fieldsRoot.className = 'metadata-record-fields'
    let removing = false
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
        if (removing) return
        const batch = removeRecordCommands(provider, schema, definition, id, check.checked)
        if (!batch) {
          detach.hidden = false
          confirm.error.textContent = tr(
            'Cette affiliation est utilisée. Confirmez le retrait de ses associations.',
          )
          return
        }
        removing = true
        void commands
          .run(batch, () => root.isConnected && !confirm.element.hidden)
          .then(() => {
            refresh()
            focusAdd(definition)
          })
          .catch((error: unknown) => {
            if (confirm.element.isConnected) confirm.error.textContent = String(error)
          })
          .finally(() => {
            removing = false
          })
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
    const up = button(
      `${tr('Monter')} · ${definition.itemLabel}`,
      'chevron-up',
      () =>
        void commands
          .run(moveRecordCommands(provider, definition, id, -1), () => root.isConnected)
          .catch(provider.onError),
    )
    const down = button(
      `${tr('Descendre')} · ${definition.itemLabel}`,
      'chevron-down',
      () =>
        void commands
          .run(moveRecordCommands(provider, definition, id, 1), () => root.isConnected)
          .catch(provider.onError),
    )
    const edit = button(`${tr('Modifier')} · ${definition.itemLabel}`, 'settings-2', () => {
      if (panel.editing === 'dialog') {
        confirm.element.hidden = true
        element.open = false
        startDraft(definition, undefined, id, edit)
        return
      }
      element.open = !element.open
      if (element.open)
        fieldsRoot
          .querySelector<HTMLElement>('input, textarea, select, [contenteditable=true]')
          ?.focus()
    })
    const add = addButton(definition, id)
    actions.append(remove, up, down, edit, add)
    summary.append(label, actions)
    if (panel.editing === 'dialog') {
      summary.setAttribute('aria-haspopup', 'dialog')
      summary.onclick = (event) => {
        event.preventDefault()
        confirm.element.hidden = true
        element.open = false
        startDraft(definition, undefined, id, summary)
      }
    }
    element.append(summary, deletion, fieldsRoot)
    collections.get(definition)!.root.append(element)
    let views: ReturnType<typeof fieldView>[] = [],
      mode = ''
    const render = () => {
      const isPrimary =
        !!definition.language_key && record.get(definition.language_key) === language()
      const nextMode = isPrimary ? 'primary' : 'other'
      if (panel.editing !== 'dialog' && mode !== nextMode) {
        for (const view of views) view.destroy()
        fieldsRoot.replaceChildren()
        mode = nextMode
        views = fields(definition)
          .filter((field) => !isPrimary || field.kind !== 'language')
          .map((field) => fieldView(fieldsRoot, definition, record, field))
      }
      const value = String(record.get(definition.title_key) ?? '')
      caption.textContent = definition.language_key
        ? catalogue.languages.find((item) => item.value === value)?.label || definition.itemLabel
        : String(value || definition.itemLabel)
      caption.title = caption.textContent
      detail.textContent = (definition.summary || [])
        .map((key) => {
          const field = definition.fields.find((item) => item.key === key)
          const value = record.get(key)
          return field?.kind === 'boolean' ? (value ? field.label : '') : String(value || '')
        })
        .filter(Boolean)
        .join(' · ')
      detail.hidden = !detail.textContent
      const ids = recordIds(provider, definition),
        index = ids.indexOf(id)
      up.disabled = index === 0
      down.disabled = index === ids.length - 1
      remove.disabled = isPrimary
      add.disabled = cannotAdd(definition)
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
  function fields(definition: RecordCollection) {
    return panel.fields
      ? panel.fields.map((key) => definition.fields.find((field) => field.key === key)!)
      : definition.fields
  }
  function cannotAdd(definition: RecordCollection) {
    return (
      !!definition.language_key &&
      recordIds(provider, definition).length >= catalogue.languages.length
    )
  }
  function focusAdd(definition: RecordCollection, after?: string) {
    addTarget(definition, after).focus()
  }
  function addTarget(definition: RecordCollection, after?: string) {
    const state = collections.get(definition)!
    const row =
      (after && state.views.get(after)) ||
      state.views.get(recordIds(provider, definition).at(-1) || '')
    return row?.element.querySelector<HTMLButtonElement>('[data-record-add]') || state.add
  }
  function addButton(definition: RecordCollection, after?: string) {
    const add = button(`${tr('Ajouter')} · ${definition.itemLabel}`, 'plus', () =>
      startDraft(definition, after, undefined, add),
    )
    add.dataset.recordAdd = ''
    return add
  }
  function startDraft(
    definition: RecordCollection,
    after?: string,
    editing?: string,
    trigger?: HTMLElement,
  ) {
    const state = collections.get(definition)!
    if (state.draft) {
      state.draft.focus()
      return
    }
    if (!editing && cannotAdd(definition)) return
    const langKey = definition.language_key
    const used = recordIds(provider, definition).map((id) =>
      recordMap(provider, definition)
        .get(id)!
        .get(langKey || ''),
    )
    const next = catalogue.languages.find((item) => !used.includes(item.value))?.value
    const draft = prepareRecordEdit(provider, schema, definition, {
      id: editing,
      after,
      defaults: langKey ? { [langKey]: next } : {},
    })
    // Rich drafts have their own local history. Only confirmation publishes one document transaction.
    const owner = fields(definition).some(
      (field) => field.kind === 'rich' || field.kind === 'rich_inline',
    )
      ? createRichZone({
          profile: 'rich-text',
          label: definition.itemLabel,
          element: document.createElement('div'),
          binding: { kind: 'document', document: draft.doc },
        })
      : undefined
    let views: ReturnType<typeof fieldView>[] = []
    let returnFocus: HTMLElement | undefined
    const discard = (saved?: string) => {
      state.draft?.destroy()
      state.draft = undefined
      refresh()
      returnFocus =
        (editing || !saved) && trigger?.isConnected
          ? trigger
          : addTarget(definition, saved || after)
      returnFocus.focus({ preventScroll: true })
    }
    const modal = panel.editing === 'dialog'
    const dialog = modal
      ? formDialog({
          id: `edit-record-${definition.parameter}`,
          title: `${tr(editing ? 'Modifier' : 'Ajouter')} · ${definition.itemLabel}`,
          cancel: () => discard(),
          returnFocus: () => returnFocus || trigger,
        })
      : undefined
    const form =
      dialog ||
      contextPanel({
        id: `add-record-${definition.parameter}`,
        closeLabel: tr('Annuler'),
        close: () => discard(),
        canConfirm: () => true,
      })
    form.title.textContent = `${tr(editing ? 'Modifier' : 'Ajouter')} · ${definition.itemLabel}`
    form.element.classList.add('metadata-record-draft')
    form.element.hidden = false
    form.element.dataset.after = after || ''
    form.body.classList.add('metadata-record-fields')
    if (!modal) state.root.append(form.element)
    views = fields(definition).map((field) =>
      fieldView(form.body, definition, draft.record, field, draft, owner || main),
    )
    form.action({ label: tr('Annuler'), run: () => discard() })
    let pending = false
    const submit = form.action({
      label: tr('Valider'),
      primary: true,
      run: () => {
        if (pending) return
        const result = draft.plan()
        if (result.error) {
          form.error.textContent = tr(
            {
              changed:
                'Cet élément a été modifié ou supprimé ailleurs. Fermez puis rouvrez le formulaire.',
              'missing-reference':
                'Une référence sélectionnée a été supprimée. Vérifiez les associations avant de valider.',
              'duplicate-language': 'Cette langue est déjà présente.',
            }[result.error],
          )
          return
        }
        pending = true
        submit.disabled = true
        const focused = document.activeElement as HTMLElement | null
        form.body.inert = true
        const active = () => state.draft?.element === form.element && form.element.isConnected
        void commands
          .run(result.commands!, () => active() && result.current!())
          .then(() => {
            if (active()) discard(result.id)
          })
          .catch((error: unknown) => {
            if (active())
              form.error.textContent = error instanceof Error ? error.message : String(error)
          })
          .finally(() => {
            pending = false
            submit.disabled = false
            form.body.inert = false
            if (
              active() &&
              (document.activeElement === document.body || document.activeElement === form.element)
            )
              focused?.focus({ preventScroll: true })
          })
      },
    })
    const update = () => {
      for (const view of views) view.refresh()
    }
    draft.record.observeDeep(update)
    state.draft = {
      element: form.element,
      floating: modal,
      refresh: update,
      focus: () =>
        form.body
          .querySelector<HTMLElement>('input, textarea, select, [contenteditable=true]')
          ?.focus(),
      destroy() {
        draft.record.unobserveDeep(update)
        for (const view of views) view.destroy()
        owner?.destroy()
        draft.destroy()
        if (dialog) dialog.destroy()
        else form.element.remove()
      },
    }
    refresh()
    if (dialog) dialog.open()
    else state.draft.focus()
  }
  for (const definition of schema.collections.filter(
    (collection) => collection.parameter === panel.collection,
  )) {
    const section = metadataSection({
      id: definition.parameter,
      label: definition.label,
      layout: 'editorial',
    })
    section.root.classList.add('metadata-record-section')
    // The tab supplies the section name; do not repeat it inside the panel.
    section.root.querySelector('legend')?.classList.add('sr-only')
    const add = addButton(definition)
    const actions = actionRow()
    actions.classList.add('metadata-record-empty-actions')
    actions.append(add)
    section.content.append(actions)
    root.append(section.root)
    collections.set(definition, { root: section.content, views: new Map(), add })
  }
  function refresh() {
    for (const [definition, state] of collections) {
      const ids = recordIds(provider, definition)
      for (const [id, view] of state.views)
        if (!ids.includes(id)) {
          view.destroy()
          state.views.delete(id)
        }
      let previous: ChildNode = state.add.parentElement!
      ;(previous as HTMLElement).hidden = ids.length > 0 || !!state.draft
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
        if (state.draft && !state.draft.floating && id === state.draft.element.dataset.after) {
          if (previous.nextSibling !== state.draft.element)
            state.root.insertBefore(state.draft.element, previous.nextSibling)
          previous = state.draft.element
        }
      }
      if (focused?.isConnected && root.contains(focused) && document.activeElement !== focused)
        focused.focus({ preventScroll: true })
      if (state.draft) {
        if (
          !state.draft.floating &&
          !ids.includes(state.draft.element.dataset.after || '') &&
          state.root.lastChild !== state.draft.element
        )
          state.root.append(state.draft.element)
        state.draft.refresh()
      }
      state.add.disabled = cannotAdd(definition)
    }
  }
  refresh()
  return {
    refresh,
    destroy() {
      for (const state of collections.values()) {
        state.draft?.destroy()
        for (const view of state.views.values()) view.destroy()
      }
      root.remove()
    },
  }
}

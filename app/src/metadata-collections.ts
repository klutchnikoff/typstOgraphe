import { hydrateForm } from './form-recovery'
import type { RecoveredItem } from './recovered-work'
import { stableJSON } from './provider'
import { bindFormOwner } from './form-drafts'
import { historyFor } from './editing-history'
import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import type { Catalogue } from './shell'
import { metadataSection } from './metadata-fields'
import { createRichZone } from './rich-zone'
import { contextPanel } from './context-panel'
import { formDialog } from './form-dialog'
import { actionRow } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import { metadataCommands } from './metadata-commands'
import { createRecordFields } from './metadata-record-fields'
import { moveRecordCommands, removeRecordCommands } from './metadata-record-commands'
import { recordCard } from './record-card'
import { sortableCards } from './sortable-cards'
import {
  prepareRecordEdit,
  setRecordField,
  recordMap,
  recordIds,
  readRecord,
  recordSummary,
  type StructuredMetadata,
  type MetadataPanel,
  type RecordCollection,
  type SharedRecord,
} from './metadata-records'
import { errorText } from './command-error'

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
  const itemLabel = (definition: RecordCollection) => panel.itemLabel || definition.itemLabel
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
      list: HTMLElement
      views: Map<string, View>
      add: HTMLButtonElement
      sorting?: ReturnType<typeof sortableCards>
      draft?: {
        element: HTMLElement
        floating: boolean
        refresh: () => void
        destroy: () => void
        focus: () => void
      }
    }
  >()
  const fieldView = createRecordFields(provider, schema, catalogue, main, language)
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
  function recordView(definition: RecordCollection, record: SharedRecord, id: string): View {
    const card = panel.editing === 'dialog' ? recordCard(id, itemLabel(definition)) : undefined
    const element = card?.element || document.createElement('details')
    element.classList.add('metadata-record')
    element.dataset.recordId = id
    const open = (value: boolean) => {
      if (element instanceof HTMLDetailsElement) element.open = value
    }
    const summary = card?.open || document.createElement('summary')
    const caption = card?.caption || document.createElement('span')
    caption.className = 'metadata-record-caption'
    const label = document.createElement('span')
    label.className = 'metadata-record-label'
    const detail = card?.detail || document.createElement('small')
    detail.className = 'metadata-record-detail'
    if (!card) label.append(caption, detail)
    const actions = card?.actions || actionRow()
    const fieldsRoot = document.createElement('div')
    fieldsRoot.className = 'metadata-record-fields'
    let removing = false
    const deletion = document.createElement('div')
    deletion.className = 'metadata-record-deletion'
    const confirm = contextPanel({
      id: `delete-record-${definition.parameter}-${id}`,
      closeLabel: tr('Annuler'),
      close: () => {
        confirm.discardRecovery()
        confirm.element.hidden = true
        remove.focus()
      },
      canConfirm: () => true,
    })
    confirm.title.textContent = `${tr('Supprimer')} · ${definition.itemLabel}`
    if (definition.removalNotice) {
      const notice = document.createElement('p')
      notice.textContent = definition.removalNotice
      confirm.body.append(notice)
    }
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
        confirm.discardRecovery()
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
            confirm.discardRecovery()
            refresh()
            focusAdd(definition)
          })
          .catch((error: unknown) => {
            if (confirm.element.isConnected) confirm.error.textContent = errorText(error)
          })
          .finally(() => {
            removing = false
          })
      },
    })
    deletion.append(confirm.element)
    const remove = button(`${tr('Supprimer')} · ${definition.itemLabel}`, 'trash-2', () => {
      collections.get(definition)!.sorting?.cancel()
      confirm.error.textContent = ''
      check.checked = false
      detach.hidden = true
      open(true)
      confirm.element.hidden = false
      confirm.trackRecovery(provider, {
        kind: 'record-deletion',
        collection: definition.parameter,
        record: id,
      })
      confirm.focus()
    })
    const up = !card
      ? button(
          `${tr('Monter')} · ${definition.itemLabel}`,
          'chevron-up',
          () =>
            void commands
              .run(moveRecordCommands(provider, definition, id, -1), () => root.isConnected)
              .catch(provider.onError),
        )
      : undefined
    const down = !card
      ? button(
          `${tr('Descendre')} · ${definition.itemLabel}`,
          'chevron-down',
          () =>
            void commands
              .run(moveRecordCommands(provider, definition, id, 1), () => root.isConnected)
              .catch(provider.onError),
        )
      : undefined
    const editRecord = () => {
      if (panel.editing === 'dialog') {
        confirm.element.hidden = true
        open(false)
        startDraft(definition, undefined, id, summary)
        return
      }
      const details = element as HTMLDetailsElement
      details.open = !details.open
      if (details.open)
        fieldsRoot
          .querySelector<HTMLElement>('input, textarea, select, [contenteditable=true]')
          ?.focus()
    }
    if (card) actions.append(remove)
    else {
      const edit = button(`${tr('Modifier')} · ${definition.itemLabel}`, 'settings-2', editRecord)
      actions.append(remove, up!, down!, edit, addButton(definition, id))
      summary.append(label, actions)
    }
    if (panel.editing === 'dialog') {
      summary.setAttribute('aria-haspopup', 'dialog')
      summary.onclick = (event) => {
        event.preventDefault()
        confirm.element.hidden = true
        open(false)
        startDraft(definition, undefined, id, summary)
      }
    }
    if (card) element.append(deletion)
    else element.append(summary, deletion, fieldsRoot)
    collections.get(definition)!.list.append(element)
    let views: ReturnType<typeof fieldView>[] = [],
      mode = ''
    const render = () => {
      const isPrimary =
        !!definition.language_key && record.get(definition.language_key) === language()
      const nextMode = isPrimary ? 'primary' : 'other'
      if (mode && mode !== nextMode && !element.contains(document.activeElement)) open(isPrimary)
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
      detail.textContent = (panel.summary || definition.summary || [])
        .map((key) => {
          const field = definition.fields.find((item) => item.key === key)
          const value = record.get(key)
          return field?.kind === 'boolean' ? (value ? field.label : '') : recordSummary(value)
        })
        .filter(Boolean)
        .join(' · ')
      if (card && detail.textContent) caption.textContent += ` (${detail.textContent})`
      if (!card) caption.title = caption.textContent
      if (card) {
        card.open.setAttribute(
          'aria-label',
          `${tr('Modifier')} · ${itemLabel(definition)} · ${caption.textContent}`,
        )
        card.handle.setAttribute(
          'aria-label',
          tr('Réordonner · {name}', { name: caption.textContent }),
        )
      }
      detail.hidden = !!card || !detail.textContent
      const ids = recordIds(provider, definition),
        index = ids.indexOf(id)
      if (up) up.disabled = index === 0
      if (down) down.disabled = index === ids.length - 1
      remove.disabled = isPrimary
      const add = element.querySelector<HTMLButtonElement>('[data-record-add]')
      if (add) add.disabled = cannotAdd(definition)
      for (const view of views) view.refresh()
    }
    open(!!definition.language_key && record.get(definition.language_key) === language())
    render()
    return {
      record,
      element,
      refresh: render,
      destroy() {
        confirm.detachRecovery()
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
    const add = button(`${tr('Ajouter')} · ${itemLabel(definition)}`, 'plus', () =>
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
    resumed?: RecoveredItem,
  ) {
    const state = collections.get(definition)!
    state.sorting?.cancel()
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
    if (resumed) {
      const data = resumed.payload.data?.data?.record
      if (!data || typeof data.id !== 'string') {
        draft.destroy()
        return
      }
      for (const field of definition.fields)
        if (field.key in data)
          setRecordField({ doc: draft.doc }, draft.record, field.key, data[field.key])
      draft.record.set('id', data.id)
    }
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
    if (owner) bindFormOwner(owner, provider)
    let views: ReturnType<typeof fieldView>[] = []
    let returnFocus: HTMLElement | undefined
    const discard = (saved?: string) => {
      form.discardRecovery()
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
    const title = () => {
      const code = langKey && String(draft.record.get(langKey))
      const name = catalogue.languages.find((item) => item.value === code)?.label || code
      return `${tr(editing ? 'Modifier' : 'Ajouter')} · ${itemLabel(definition)}${editing && name ? ` · ${name}` : ''}`
    }
    const dialog = modal
      ? formDialog({
          id: `edit-record-${definition.parameter}`,
          title: title(),
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
    form.title.textContent = title()
    form.element.classList.add('metadata-record-draft')
    form.element.hidden = false
    form.element.dataset.after = after || ''
    form.body.classList.add('metadata-record-fields')
    if (!modal) state.list.append(form.element)
    if (dialog && owner) dialog.dialog.classList.add('rich-record-dialog')
    views = fields(definition)
      .filter(
        (field) =>
          !(
            editing &&
            langKey &&
            field.key === langKey &&
            draft.record.get(langKey) === language()
          ),
      )
      .map((field) => fieldView(form.body, definition, draft.record, field, draft, owner || main))
    if (modal && editing && owner)
      form.body.querySelector<HTMLElement>('[contenteditable=true]')!.dataset.initialFocus = ''
    form.recoveryData = () => ({ record: draft.value() })
    form.trackRecovery(provider, {
      kind: 'record',
      panel: panel.id,
      collection: definition.parameter,
      record: editing || null,
      after: after || null,
      base: resumed?.payload.context?.base || draft.base || draft.value(),
    })
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
        const focused = document.activeElement as HTMLElement | null
        pending = true
        submit.disabled = true
        form.body.inert = true
        const active = () => state.draft?.element === form.element && form.element.isConnected
        void commands
          .run(result.commands!, () => active() && result.current!())
          .then(() => {
            if (active()) {
              form.committed()
              discard(result.id)
            }
          })
          .catch((error: unknown) => {
            if (active())
              form.error.textContent = error instanceof Error ? error.message : errorText(error)
          })
          .finally(() => {
            pending = false
            submit.disabled = false
            form.body.inert = false
            if (
              active() &&
              (document.activeElement === document.body ||
                document.activeElement === form.element ||
                document.activeElement === dialog?.dialog)
            )
              focused?.focus({ preventScroll: true })
          })
      },
    })
    const update = () => {
      form.draftChanged()
      form.title.textContent = title()
      for (const view of views) view.refresh()
    }
    draft.record.observeDeep(update)
    state.draft = {
      element: form.element,
      floating: modal,
      refresh: update,
      focus: () => form.focus(),
      destroy() {
        form.detachRecovery()
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
    if (resumed) hydrateForm(provider, form.element, resumed)
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
    const cards = panel.editing === 'dialog'
    section.root.classList.toggle('metadata-card-section', cards)
    // The tab supplies the section name; do not repeat it inside the panel.
    section.root.querySelector('legend')?.classList.add('sr-only')
    const add = addButton(definition)
    const actions = actionRow()
    actions.classList.add('metadata-record-empty-actions')
    actions.append(add)
    const list = document.createElement('div')
    list.className = cards ? 'collection-card-list' : 'metadata-record-list'
    section.content.append(list, actions)
    root.append(section.root)
    const sorting = cards
      ? sortableCards(list, {
          order: () => recordIds(provider, definition),
          move: (id, before, base) =>
            commands.run(
              [
                {
                  id: 'record.move',
                  params: { collection: definition.parameter, id, ...(before ? { before } : {}) },
                },
              ],
              () =>
                root.isConnected &&
                stableJSON(recordIds(provider, definition)) === stableJSON(base),
            ),
          error: provider.onError,
        })
      : undefined
    collections.set(definition, { root: section.content, list, views: new Map(), add, sorting })
  }
  function refresh() {
    for (const [definition, state] of collections) {
      const ids = recordIds(provider, definition)
      state.sorting?.refresh()
      for (const [id, view] of state.views)
        if (!ids.includes(id)) {
          view.destroy()
          state.views.delete(id)
        }
      let previous: ChildNode | null = null
      state.add.parentElement!.hidden =
        panel.editing !== 'dialog' && (ids.length > 0 || !!state.draft)
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
        const next: ChildNode | null = previous ? previous.nextSibling : state.list.firstChild
        if (!state.sorting?.active() && next !== view.element)
          state.list.insertBefore(view.element, next)
        previous = view.element
        if (state.draft && !state.draft.floating && id === state.draft.element.dataset.after) {
          if (previous.nextSibling !== state.draft.element)
            state.list.insertBefore(state.draft.element, previous.nextSibling)
          previous = state.draft.element
        }
      }
      if (focused?.isConnected && root.contains(focused) && document.activeElement !== focused)
        focused.focus({ preventScroll: true })
      if (state.draft) {
        if (
          !state.draft.floating &&
          !ids.includes(state.draft.element.dataset.after || '') &&
          state.list.lastChild !== state.draft.element
        )
          state.list.append(state.draft.element)
        state.draft.refresh()
      }
      state.add.disabled = cannotAdd(definition)
      state.sorting?.refresh()
    }
  }
  refresh()
  return {
    resume(item: RecoveredItem) {
      const context = item.payload.context
      if (context.panel && context.panel !== panel.id) return false
      const definition = schema.collections.find((c) => c.parameter === context?.collection)
      if (!definition || !collections.has(definition)) return false
      if (context.record) {
        const record = recordMap(provider, definition).get(context.record)
        if (!record || stableJSON(readRecord(record, definition)) !== stableJSON(context.base))
          return false
      } else if (recordMap(provider, definition).has(item.payload.data?.data?.record?.id))
        return false
      startDraft(
        definition,
        context.after || undefined,
        context.record || undefined,
        undefined,
        item,
      )
      return !!collections.get(definition)?.draft
    },
    refresh,
    destroy() {
      for (const state of collections.values()) {
        state.sorting?.destroy()
        state.draft?.destroy()
        for (const view of state.views.values()) view.destroy()
      }
      root.remove()
    },
  }
}

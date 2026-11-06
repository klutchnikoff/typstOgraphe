import type { Editor } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import { stableJSON } from './provider'
import { metadataCommands } from './metadata-commands'
import { formDialog } from './form-dialog'
import { bibliographyRequests } from './bibliography-requests'
import { bibliographyFields, diagnosticText } from './bibliography-fields'
import {
  importReview,
  refreshImportPlan,
  reviewImport,
  type ImportDraft,
} from './bibliography-import'
import type { Browse, Form, Draft, Notice, Source, Report } from './bibliography-types'
import { profiles } from './bibliography-types'
import { labeledField } from './form-field'
import { referenceRecords } from './citation-index'
import { documentContent } from './shared-inline-marks'
import { positionAtPath } from './editor-targets'
import { consumeRecovered, type RecoveredItem } from './recovered-work'
import { bibliographyBrowser } from './bibliography-browser'
import { tr } from './i18n'
import { errorText } from './command-error'
import { prepareBibliographyPublication, type CitationDraft } from './bibliography-publication'
interface Options {
  create?: boolean
  cite?: CitationDraft
  done?: () => void
  recovered?: RecoveredItem
}
interface OpenManager {
  focus(): void
  resume(item: RecoveredItem): void
}
const managers = new WeakMap<Provider, OpenManager>()
export function resumeBibliography(
  provider: Provider,
  editor: Editor,
  transport: Transport,
  item: RecoveredItem,
) {
  openBibliography(editor, provider, transport, { recovered: item })
  return true
}
/** A private form over frozen native data; every publication uses the shared document engine. */
export function openBibliography(
  editor: Editor,
  provider: Provider,
  transport: Transport,
  options: Options = {},
) {
  if (managers.has(provider)) {
    if (options.recovered) managers.get(provider)!.resume(options.recovered)
    else managers.get(provider)!.focus()
    return
  }
  let alive = true,
    busy = false,
    browsing = false,
    draft: Draft | undefined,
    imported: ImportDraft | undefined,
    baseline = '',
    recovered = options.recovered,
    cite = options.cite
  let cleanupFields = () => {}
  let lastLibrary = stableJSON(referenceRecords(provider))
  const requests = bibliographyRequests(provider, editor, transport)
  const { current, query: api } = requests
  const initialFocus = document.activeElement as HTMLElement
  const form = formDialog({
    id: 'bibliography-manager',
    title: tr('Bibliothèque du projet'),
    cancel: () => leave(() => destroy()),
    returnFocus: () => (initialFocus?.isConnected ? initialFocus : editor.view.dom),
  })
  form.dialog.classList.add('bibliography-dialog')
  const browser = bibliographyBrowser(form.body, (id) => {
    if (!browsing && !busy) leave(() => void select(id, true))
  })
  const { right, list, search, add, load, file } = browser
  const remove = form.action({
    label: tr('Supprimer'),
    danger: true,
    run: () => void removeNotice(),
  })
  const cancel = form.action({ label: tr('Annuler'), run: () => leave(() => destroy()) })
  const save = form.action({ label: tr('Valider'), primary: true, run: () => void publish() })
  const close = form.action({ label: tr('Fermer'), run: () => leave(() => destroy()) })
  const message = (error: unknown) => {
    if (current())
      form.error.textContent = error instanceof Error ? error.message : errorText(error)
  }
  const data = () => ({ draft, imported, cite, query: search.value })
  const dirty = () => !!imported || (!!draft && stableJSON(draft) !== baseline)
  function changed() {
    form.draftChanged()
    refreshActions()
  }
  function refreshActions() {
    remove.hidden = !draft || draft.creating || !!imported
    remove.disabled = busy
    save.hidden = !draft && !imported
    save.disabled =
      busy ||
      (!!imported &&
        (imported.report.plan.blocked || imported.ready !== stableJSON(imported.decisions)))
    save.textContent = tr(
      imported
        ? 'Importer'
        : cite
          ? draft?.creating
            ? 'Créer et citer'
            : 'Valider et citer'
          : 'Valider',
    )
    cancel.disabled = close.disabled = busy
    cancel.hidden = !dirty()
    close.hidden = dirty()
    load.disabled = add.disabled = busy || browsing
    form.element.querySelector<HTMLButtonElement>('.panel-title button')!.disabled = busy
  }
  function track() {
    form.detachRecovery()
    form.recoveryData = data
    form.trackRecovery(provider, { kind: 'bibliography' })
    if (recovered) {
      form.bindRecoverySource(recovered.id)
      const sourceId = recovered.id
      form.recoveryApplied = () => consumeRecovered(provider, sourceId)
    }
    changed()
  }
  function clear() {
    form.discardRecovery()
    draft = undefined
    imported = undefined
    baseline = ''
    cleanupFields()
    cleanupFields = () => {}
    right.replaceChildren()
    refreshActions()
  }
  function onEditorDestroy() {
    destroy()
  }
  function destroy() {
    editor.off('destroy', onEditorDestroy)
    if (!alive) return
    alive = false
    cleanupFields()
    requests.destroy()
    provider.doc.getMap('settings').unobserveDeep(observe)
    browser.destroy()
    managers.delete(provider)
    form.destroy()
  }
  function leave(action: () => void) {
    if (busy || form.composing) return
    if (!dirty()) {
      clear()
      action()
      return
    }
    const previousFocus = document.activeElement as HTMLElement
    const question = formDialog({
      id: 'bibliography-leave',
      title: tr('Conserver les modifications ?'),
      cancel: () => question.destroy(),
      returnFocus: () => (previousFocus?.isConnected ? previousFocus : form.element),
    })
    question.body.textContent = tr('Cette saisie n’a pas encore été appliquée au document.')
    question.action({
      label: tr('Abandonner'),
      danger: true,
      run: () => {
        question.destroy()
        if (recovered) void consumeRecovered(provider, recovered.id).catch(message)
        recovered = undefined
        clear()
        action()
      },
    })
    question.action({ label: tr('Continuer l’édition'), run: () => question.destroy() })
    question.action({
      label: tr('Valider et poursuivre'),
      primary: true,
      run: async () => {
        question.destroy()
        if (await publish(false)) action()
      },
    })
    question.open()
  }
  async function browse() {
    const latest = requests.latest('browse')
    browsing = true
    refreshActions()
    list.inert = true
    list.setAttribute('aria-busy', 'true')
    try {
      const result = await api<Browse>('bibliography.browse', { query: search.value })
      if (!latest()) return
      list.inert = false
      list.setAttribute('aria-busy', 'false')
      browser.render(result, draft?.base.id)
      if (!draft && !imported && result.entries.length) await select(result.entries[0].id, false)
    } catch (error) {
      if (latest()) {
        list.inert = false
        list.setAttribute('aria-busy', 'false')
        list.replaceChildren()
        message(error)
      }
    } finally {
      if (latest()) {
        browsing = false
        refreshActions()
      }
    }
  }
  function show() {
    browser.select(draft?.base.id)
    cleanupFields()
    cleanupFields = () => {}
    right.replaceChildren()
    form.error.textContent = ''
    if (imported) {
      importReview(right, imported, referenceRecords(provider), () => {
        changed()
        void resolve()
      })
      refreshActions()
      return
    }
    if (!draft) {
      right.textContent = tr('Créez ou importez une notice pour commencer.')
      refreshActions()
      return
    }
    const selected = draft
    const header = document.createElement('div'),
      profile = document.createElement('select')
    header.className = 'bibliography-notice-heading'
    for (const id of draft.profiles) profile.add(new Option(tr(profiles[id] || id), id))
    profile.value = draft.profile
    profile.id = 'bibliography-profile'
    profile.onchange = () => {
      const next = profile.value
      profile.value = selected.profile
      void changeProfile(next)
    }
    header.append(labeledField(tr('Type de notice'), profile))
    right.append(header)
    cleanupFields = bibliographyFields(right, draft.fields, draft.values, changed)
    const key = document.createElement('input')
    key.id = 'bibliography-key'
    key.value = draft.key
    key.placeholder = draft.working.key
    key.oninput = () => {
      selected.key = key.value
      changed()
    }
    const identifiers = right.querySelector('details') || right
    identifiers.append(labeledField(tr('Clé de citation'), key))
    const hints = document.createElement('p')
    hints.className = 'bibliography-completeness'
    hints.textContent = tr('Les champs conseillés manquants n’empêchent pas la validation.')
    right.append(hints)
    const usage = document.createElement('button')
    usage.type = 'button'
    usage.textContent = tr('Voir les citations')
    usage.hidden = draft.creating
    usage.onclick = () => void findCitations()
    right.append(usage)
    const usageCurrent = requests.latest('usage')
    if (!draft.creating)
      void api<{ targets: { path: number[] }[] }>('bibliography.usage', { id: draft.base.id })
        .then((result) => {
          if (usageCurrent() && draft === selected) {
            usage.textContent = tr('Voir les citations ({count})', { count: result.targets.length })
            usage.disabled = !result.targets.length
          }
        })
        .catch((error: unknown) => {
          if (usageCurrent() && draft === selected) message(error)
        })
    refreshActions()
  }
  async function select(id: string, focus: boolean) {
    if (busy) return
    busy = true
    right.inert = true
    refreshActions()
    try {
      const record = await api<{ record: Notice; fingerprint: string }>('bibliography.record', {
        id,
      })
      const fields = await api<Form>('bibliography.form', { record: record.record })
      if (!current()) return
      clear()
      draft = {
        base: record.record,
        working: record.record,
        profile: fields.profile,
        profiles: fields.profiles,
        fingerprint: record.fingerprint,
        fields: fields.fields,
        values: {},
        key: record.record.key,
        creating: false,
      }
      show()
      baseline = stableJSON(draft)
      track()
      if (focus) right.querySelector<HTMLInputElement>('input')?.focus()
    } catch (error) {
      message(error)
    } finally {
      busy = false
      right.inert = false
      refreshActions()
      if (focus && current()) right.querySelector<HTMLInputElement>('input')?.focus()
    }
  }
  async function create(profile = 'journal_article') {
    if (busy) return
    busy = true
    right.inert = true
    refreshActions()
    try {
      const record = await api<{ record: Notice; fingerprint: string }>(
        'bibliography.prepare_record',
        { id: crypto.randomUUID(), profile },
      )
      const fields = await api<Form>('bibliography.form', { record: record.record })
      if (!current()) return
      clear()
      draft = {
        base: record.record,
        working: record.record,
        profile: fields.profile,
        profiles: fields.profiles,
        fingerprint: record.fingerprint,
        fields: fields.fields,
        values: {},
        key: '',
        creating: true,
      }
      show()
      baseline = stableJSON(draft)
      track()
      right.querySelector<HTMLInputElement>('input[data-initial-focus]')?.focus()
    } catch (error) {
      message(error)
    } finally {
      busy = false
      right.inert = false
      refreshActions()
      if (current()) right.querySelector<HTMLInputElement>('input[data-initial-focus]')?.focus()
    }
  }
  async function changeProfile(profile: string) {
    const selected = draft
    if (!selected || busy || form.composing || !current()) return
    busy = true
    right.inert = true
    refreshActions()
    form.error.textContent = ''
    const frozen = stableJSON(selected)
    try {
      const prepared = await api<Form>('bibliography.prepare_form', {
        record: selected.working,
        values: selected.values,
        key: selected.key || null,
        profile,
      })
      if (!current() || draft !== selected || stableJSON(selected) !== frozen) return
      selected.working = prepared.record
      selected.profile = prepared.profile
      selected.profiles = prepared.profiles
      selected.fields = prepared.fields
      selected.values = {}
      show()
      changed()
    } catch (error) {
      if (current()) message(error)
    } finally {
      busy = false
      right.inert = false
      refreshActions()
      right
        .querySelector<HTMLSelectElement>('#bibliography-profile')
        ?.focus({ preventScroll: true })
    }
  }
  async function publish(finish = true): Promise<boolean> {
    if (busy || form.composing || !current()) return false
    const selected = draft,
      review = imported
    if (!selected && !review) return false
    const frozen = stableJSON({ draft, imported, cite }),
      focused = document.activeElement as HTMLElement
    busy = true
    right.inert = true
    refreshActions()
    form.error.textContent = ''
    try {
      const batch = await prepareBibliographyPublication(
        api,
        structuredClone(selected),
        structuredClone(review),
        structuredClone(cite),
      )
      if (!batch) return false
      if (cite && stableJSON(documentContent(editor.getJSON())) !== stableJSON(cite.document))
        throw new Error(
          tr('La cible de citation a changé ; votre notice est conservée dans ce formulaire.'),
        )
      if (!current() || draft !== selected || imported !== review) return false
      await metadataCommands(editor, provider).run(
        batch,
        () =>
          current() &&
          stableJSON({ draft, imported, cite }) === frozen &&
          draft === selected &&
          imported === review &&
          (!cite || stableJSON(documentContent(editor.getJSON())) === stableJSON(cite.document)),
      )
      form.committed()
      recovered = undefined
      const id = selected?.base.id,
        position = cite?.position
      clear()
      busy = false
      if (cite) {
        destroy()
        if (position !== undefined) editor.commands.setTextSelection(position + 1)
        options.done?.()
        editor.view.focus()
      } else if (finish) {
        await browse()
        if (id) await select(id, false)
      }
      return true
    } catch (error) {
      message(error)
      return false
    } finally {
      busy = false
      right.inert = false
      refreshActions()
      if (current() && focused?.isConnected) focused.focus({ preventScroll: true })
    }
  }
  async function removeNotice() {
    const selected = draft
    if (!selected || selected.creating || busy) return
    const question = formDialog({
      id: 'bibliography-remove',
      title: tr('Supprimer cette notice ?'),
      cancel: () => question.destroy(),
      returnFocus: () => remove,
    })
    question.body.textContent = selected.base.key
    question.action({ label: tr('Annuler'), run: () => question.destroy() })
    question.action({
      label: tr('Supprimer'),
      danger: true,
      primary: true,
      run: async () => {
        if (busy) return
        busy = true
        refreshActions()
        try {
          const inspection = await api<{ fingerprint: string }>('bibliography.inspect')
          await metadataCommands(editor, provider).run(
            [
              {
                id: 'bibliography.apply',
                params: {
                  expected: inspection.fingerprint,
                  change: { kind: 'remove', id: selected.base.id, expected: selected.fingerprint },
                },
              },
            ],
            () => current() && draft === selected,
          )
          form.committed()
          recovered = undefined
          question.destroy()
          clear()
          busy = false
          await browse()
        } catch (error) {
          question.error.textContent = error instanceof Error ? error.message : errorText(error)
        } finally {
          busy = false
          refreshActions()
        }
      },
    })
    question.open()
  }
  async function findCitations() {
    if (!draft) return
    const selected = draft
    const latest = requests.latest('navigation')
    try {
      const result = await api<{ targets: { path: number[] }[] }>('bibliography.usage', {
        id: selected.base.id,
      })
      const target = result.targets[0]
      if (target && latest() && draft === selected)
        leave(() => {
          destroy()
          const position = positionAtPath(editor, target.path)
          editor.commands.setTextSelection(position + 1)
          editor.view.focus()
        })
    } catch (error) {
      if (latest() && draft === selected) message(error)
    }
  }
  async function resolve() {
    const selected = imported
    if (!selected) return
    const latest = requests.latest('review')
    refreshActions()
    try {
      const result = await reviewImport(provider, transport, selected)
      if (latest() && imported === selected) {
        selected.report.plan = result.plan
        refreshImportPlan(right, result.plan)
        selected.ready = stableJSON(selected.decisions)
        form.error.textContent = result.plan.blocked
          ? tr('Cet import nécessite une décision de rapprochement.')
          : ''
        changed()
      }
    } catch (error) {
      if (latest()) message(error)
    }
  }
  file.onchange = async () => {
    const selected = file.files?.[0]
    file.value = ''
    if (!selected || busy) return
    busy = true
    refreshActions()
    try {
      if (selected.size > 2 * 1024 * 1024)
        throw new Error(tr('Fichier trop volumineux (2 Mio maximum).'))
      const source: Source = {
        id: crypto.randomUUID(),
        name: selected.name,
        format: selected.name.toLowerCase().endsWith('.bib') ? 'biblatex' : 'hayagriva',
        contents: new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(
          await selected.arrayBuffer(),
        ),
      }
      busy = false
      refreshActions()
      if (!current()) return
      leave(() => {
        busy = true
        refreshActions()
        void api<Report>(
          'bibliography.prepare_import',
          source as unknown as Record<string, unknown>,
        )
          .then((report) => {
            if (current()) {
              clear()
              imported = { source, report, decisions: [], ready: stableJSON([]) }
              show()
              track()
              form.error.textContent = report.plan.blocked
                ? tr('Cet import nécessite une décision de rapprochement.')
                : ''
            }
          })
          .catch(message)
          .finally(() => {
            busy = false
            refreshActions()
          })
      })
    } catch (error) {
      busy = false
      refreshActions()
      if (current()) message(error)
    }
  }
  function observe() {
    if (!current()) {
      destroy()
      return
    }
    const library = stableJSON(referenceRecords(provider))
    if (library !== lastLibrary) {
      lastLibrary = library
      void browse()
      if (
        draft &&
        !draft.creating &&
        stableJSON(referenceRecords(provider).find((r) => r.id === draft!.base.id)) !==
          stableJSON(draft.base)
      )
        form.error.textContent = tr(
          'Cette notice a été modifiée ailleurs. Votre saisie est conservée.',
        )
    }
  }
  function resume(item: RecoveredItem) {
    const saved = item.payload.data?.data
    if (!saved?.draft && !saved?.imported) return
    recovered = item
    clear()
    draft = structuredClone(saved.draft)
    imported = structuredClone(saved.imported)
    cite = structuredClone(saved.cite)
    search.value = saved.query || ''
    show()
    baseline = ''
    track()
    form.error.textContent = tr('Saisie récupérée · à valider')
    void browse()
    if (imported) void resolve()
  }
  add.onclick = () => leave(() => void create())
  load.onclick = () => {
    file.click()
  }
  search.oninput = () => void browse()
  provider.doc.getMap('settings').observeDeep(observe)
  editor.on('destroy', onEditorDestroy)
  managers.set(provider, {
    focus: () => search.focus(),
    resume: (item) => leave(() => resume(item)),
  })
  form.open()
  refreshActions()
  if (recovered) resume(recovered)
  else if (options.create) void create().then(() => browse())
  else void browse()
}

import { comparisonView } from './comparison-view'
import type { ComparisonAPI, ComparisonOrigin, ComparisonState } from './comparison-types'
import { contextPanel } from './context-panel'
import { keyboardScope, navigationGroup, actionRow } from './keyboard-navigation'
import { versionContent } from './version-content'
import { versionFields } from './version-fields'
import type { VersionAPI, VersionCatalogue, VersionRecord } from './version-types'
import type { Catalogue } from './shell'
import { tr } from './i18n'
import { icon } from './icons'

/** Read-only state viewer. Band decisions are separate from the displayed immutable state. */
export function versionDialog(options: {
  record: VersionRecord
  index: VersionCatalogue
  api: VersionAPI
  catalogue: Catalogue
  returnFocus(): HTMLElement | undefined
  changed(index: VersionCatalogue): void
}) {
  return stateDialog({
    version: options,
    catalogue: options.catalogue,
    returnFocus: options.returnFocus,
  })
}
export function proposalDialog(options: {
  id: string
  title: string
  api: ComparisonAPI
  catalogue: Catalogue
  returnFocus(): HTMLElement | undefined
}) {
  return stateDialog({
    proposal: options,
    catalogue: options.catalogue,
    returnFocus: options.returnFocus,
  })
}
function stateDialog(options: {
  version?: {
    record: VersionRecord
    index: VersionCatalogue
    api: VersionAPI
    changed(index: VersionCatalogue): void
  }
  proposal?: { id: string; title: string; api: ComparisonAPI }
  catalogue: Catalogue
  returnFocus(): HTMLElement | undefined
}) {
  const dialog = document.createElement('dialog')
  dialog.className = 'app-dialog form-dialog version-dialog'
  dialog.setAttribute('aria-labelledby', 'version-dialog-title')
  const header = document.createElement('header'),
    heading = document.createElement('div')
  const title = document.createElement('h2'),
    date = document.createElement('time')
  title.id = 'version-dialog-title'
  title.textContent = options.version?.record.details.name || options.proposal!.title
  if (options.version) {
    date.dateTime = options.version.record.created_at
    date.textContent = new Date(options.version.record.created_at).toLocaleString(
      options.catalogue.locale,
    )
  } else date.hidden = true
  const details = document.createElement('p')
  details.className = 'version-description'
  const showDetails = () => {
    if (!record) {
      details.hidden = true
      return
    }
    const info = record.details
    details.textContent = [
      info.tags.join(' · '),
      info.creator ? `${tr('Créée par')} ${info.creator}` : '',
      info.description || '',
    ]
      .filter(Boolean)
      .join(' — ')
    details.hidden = !details.textContent
  }
  heading.append(title, date, details)
  const button = (label: string, glyph?: string) => {
    const b = document.createElement('button')
    b.type = 'button'
    if (glyph) {
      b.innerHTML = icon(glyph)
      b.setAttribute('aria-label', tr(label))
      b.title = tr(label)
      b.className = 'icon-button'
    } else b.textContent = tr(label)
    return b
  }
  const edit = button('Modifier les informations de la version', 'pencil')
  const close = button('Fermer la version', 'x')
  edit.hidden = !options.version
  header.append(heading, edit, close)
  const tabs = document.createElement('nav')
  tabs.className = 'version-tabs'
  tabs.setAttribute('aria-label', tr('Affichage de la version'))
  tabs.setAttribute('role', 'tablist')
  const display = document.createElement('div')
  display.className = 'version-display'
  display.id = 'version-display'
  display.tabIndex = 0
  display.setAttribute('role', 'tabpanel')
  const bandSlot = document.createElement('section')
  const footer = actionRow()
  footer.classList.add('version-footer')
  const remove = button('Supprimer cette version', 'trash-2')
  remove.classList.add('danger')
  remove.hidden = !options.version
  const done = button('Fermer')
  done.className = 'primary'
  footer.append(remove, done)
  const error = document.createElement('p')
  error.className = 'version-error'
  error.setAttribute('role', 'alert')
  dialog.append(header, tabs, display, bandSlot, error, footer)
  const comparisonAPI = options.version?.api.comparison || options.proposal?.api
  let comparison: ComparisonState | undefined,
    loading: Promise<ComparisonState> | undefined,
    against: string | undefined
  let record = options.version?.record,
    index = options.version?.index,
    view = options.proposal ? 'compare' : 'content',
    ticket = 0,
    alive = true
  let disposeContent: (() => void) | undefined,
    band: ReturnType<typeof contextPanel> | undefined,
    busy = false
  const tabButtons = ['Contenu', 'Aperçu', ...(comparisonAPI ? ['Comparer'] : [])].map(
    (label, i) => {
      const b = button(label)
      b.dataset.view = ['content', 'preview', 'compare'][i]
      b.id = `version-tab-${b.dataset.view}`
      b.setAttribute('role', 'tab')
      b.setAttribute('aria-controls', display.id)
      b.onclick = () => {
        view = b.dataset.view!
        void render()
      }
      tabs.append(b)
      return b
    },
  )
  navigationGroup(tabs, { axis: 'horizontal', roving: true, activateOnMove: true })
  keyboardScope(dialog, {
    close: () => (band ? cancelBand() : destroy()),
    tab: true,
    tabLoop: true,
    priority: 40,
  })
  function cancelBand() {
    if (busy) return
    const target = band?.element.dataset.kind === 'delete' ? remove : edit
    band?.element.remove()
    band = undefined
    tabs.inert = display.inert = false
    footer.hidden = false
    edit.disabled = false
    target.focus({ preventScroll: true })
  }
  async function frozenComparison() {
    if (comparison) return comparison
    if (!loading) {
      const origin: ComparisonOrigin = options.proposal
        ? { kind: 'proposal', id: options.proposal.id }
        : { kind: 'version', id: record!.id, ...(against ? { against } : {}) }
      loading = comparisonAPI!
        .open(origin)
        .then((value) => {
          if (!alive) void comparisonAPI!.close(value.id).catch(() => {})
          else comparison = value
          return value
        })
        .catch((reason) => {
          loading = undefined
          throw reason
        })
    }
    return loading
  }
  async function render() {
    const request = ++ticket
    error.textContent = ''
    for (const b of tabButtons) {
      b.setAttribute('aria-selected', String(b.dataset.view === view))
      b.tabIndex = b.dataset.view === view ? 0 : -1
    }
    display.setAttribute('aria-labelledby', `version-tab-${view}`)
    display.setAttribute('aria-busy', 'true')
    try {
      if (view === 'compare') {
        const value = await frozenComparison()
        if (!alive || request !== ticket) return
        const picker = document.createElement('label')
        picker.className = 'comparison-picker'
        picker.append(document.createTextNode(tr('Comparer à')))
        const select = document.createElement('select')
        const current = document.createElement('option')
        current.value = ''
        current.textContent = tr('Document courant · état figé')
        select.append(current)
        for (const item of index?.versions || []) {
          if (item.id === record?.id) continue
          const option = document.createElement('option')
          option.value = item.id
          option.textContent = item.details.name
          select.append(option)
        }
        select.value = against || ''
        select.onchange = () => {
          const previous = comparison
          against = select.value || undefined
          comparison = undefined
          loading = undefined
          disposeContent?.()
          disposeContent = undefined
          if (previous) void comparisonAPI!.close(previous.id).catch(() => {})
          void render()
        }
        picker.append(select)
        picker.hidden = !!options.proposal
        const rendered = comparisonView({
          state: value,
          api: comparisonAPI!,
          catalogue: options.catalogue,
          picker: options.proposal ? undefined : picker,
          error: (reason) => {
            error.textContent = String(reason)
          },
        })
        disposeContent?.()
        disposeContent = rendered.destroy
        display.replaceChildren(rendered.element)
      } else if (view === 'content') {
        const value = options.version
          ? await options.version.api.read(record!.id)
          : (await frozenComparison()).after
        if (!alive || request !== ticket) return
        const rendered = versionContent(value, options.catalogue)
        disposeContent?.()
        disposeContent = rendered.destroy
        display.replaceChildren(rendered.element)
      } else {
        const preview = options.version
          ? await options.version.api.preview(record!.id)
          : await comparisonAPI!.preview((await frozenComparison()).id, 'after')
        if (!alive || request !== ticket) return
        const pages = preview.pages.map((src, i) => {
          const image = document.createElement('img')
          image.src = src
          image.alt = `${tr('Page')} ${i + 1}`
          image.className = 'version-page'
          return image
        })
        disposeContent?.()
        disposeContent = undefined
        display.replaceChildren(...pages)
      }
    } catch (reason) {
      if (!alive || request !== ticket) return
      disposeContent?.()
      disposeContent = undefined
      display.replaceChildren()
      error.textContent = String(reason)
    } finally {
      if (alive && request === ticket) display.removeAttribute('aria-busy')
    }
  }
  function openBand(kind: 'metadata' | 'delete') {
    if (!options.version || !record || !index) return
    if (band) cancelBand()
    band = contextPanel({
      id: 'version-decision',
      closeLabel: tr('Annuler'),
      close: cancelBand,
      canConfirm: () => !busy,
      modal: true,
    })
    const form = band
    form.element.hidden = false
    form.element.dataset.kind = kind
    form.title.textContent = tr(
      kind === 'delete' ? 'Supprimer cette version ?' : 'Informations de la version',
    )
    const fields = kind === 'metadata' ? versionFields(record!.details) : undefined
    if (fields) form.body.append(fields.element)
    else {
      const text = document.createElement('p')
      text.textContent = tr(
        'Seule cette version sera supprimée. Le document courant et les autres versions seront conservés.',
      )
      form.body.append(text)
    }
    const submit = async () => {
      if (busy || (fields && !fields.valid())) return
      busy = true
      const controls = [...form.element.querySelectorAll<HTMLButtonElement>('button')]
      controls.forEach((b) => (b.disabled = true))
      form.error.textContent = ''
      try {
        const action =
          kind === 'delete'
            ? { kind: 'delete' as const, id: record!.id }
            : { kind: 'update' as const, id: record!.id, details: fields!.value() }
        const next = await options.version!.api.change(index!.history, action)
        if (!next || !alive) return
        index = next
        options.version!.changed(next)
        if (kind === 'delete') {
          busy = false
          destroy()
          return
        }
        record = next.versions.find((v) => v.id === record!.id)!
        title.textContent = record.details.name
        showDetails()
        busy = false
        cancelBand()
      } catch (reason) {
        form.error.textContent = String(reason)
      } finally {
        busy = false
        controls.forEach((b) => (b.disabled = false))
      }
    }
    if (kind === 'delete') {
      form.action({ label: tr('Supprimer'), danger: true, run: () => void submit() })
      form.action({ label: tr('Annuler'), primary: true, run: cancelBand })
    } else {
      form.action({ label: tr('Annuler'), run: cancelBand })
      form.action({ label: tr('Valider'), primary: true, run: () => void submit() })
    }
    tabs.inert = display.inert = true
    footer.hidden = true
    edit.disabled = true
    bandSlot.append(form.element)
    form.focus()
    if (!fields) form.actions.querySelector<HTMLButtonElement>('.primary')!.focus()
  }
  function destroy(force = false) {
    if (!alive || (busy && !force)) return
    alive = false
    ticket++
    disposeContent?.()
    if (comparison) void comparisonAPI?.close(comparison.id).catch(() => {})
    dialog.close()
    dialog.remove()
    options.returnFocus()?.focus({ preventScroll: true })
  }
  dialog.addEventListener('cancel', (e) => {
    e.preventDefault()
    if (band) cancelBand()
    else destroy()
  })
  edit.onclick = () => openBand('metadata')
  remove.onclick = () => openBand('delete')
  close.onclick = done.onclick = () => (band ? cancelBand() : destroy())
  showDetails()
  document.body.append(dialog)
  dialog.showModal()
  done.focus()
  void render()
  return { destroy }
}

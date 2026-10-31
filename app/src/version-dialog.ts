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
  const dialog = document.createElement('dialog')
  dialog.className = 'app-dialog form-dialog version-dialog'
  dialog.setAttribute('aria-labelledby', 'version-dialog-title')
  const header = document.createElement('header'),
    heading = document.createElement('div')
  const title = document.createElement('h2'),
    date = document.createElement('time')
  title.id = 'version-dialog-title'
  title.textContent = options.record.details.name
  date.dateTime = options.record.created_at
  date.textContent = new Date(options.record.created_at).toLocaleString(options.catalogue.locale)
  const details = document.createElement('p')
  details.className = 'version-description'
  const showDetails = () => {
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
  const done = button('Fermer')
  done.className = 'primary'
  footer.append(remove, done)
  const error = document.createElement('p')
  error.className = 'version-error'
  error.setAttribute('role', 'alert')
  dialog.append(header, tabs, display, bandSlot, error, footer)
  let record = options.record,
    index = options.index,
    view = 'content',
    ticket = 0,
    alive = true
  let disposeContent: (() => void) | undefined,
    band: ReturnType<typeof contextPanel> | undefined,
    busy = false
  const tabButtons = ['Contenu', 'Aperçu'].map((label, i) => {
    const b = button(label)
    b.dataset.view = i ? 'preview' : 'content'
    b.id = `version-tab-${b.dataset.view}`
    b.setAttribute('role', 'tab')
    b.setAttribute('aria-controls', display.id)
    b.onclick = () => {
      view = b.dataset.view!
      void render()
    }
    tabs.append(b)
    return b
  })
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
      if (view === 'content') {
        const value = await options.api.read(record.id)
        if (!alive || request !== ticket) return
        const rendered = versionContent(value, options.catalogue)
        disposeContent?.()
        disposeContent = rendered.destroy
        display.replaceChildren(rendered.element)
      } else {
        const preview = await options.api.preview(record.id)
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
    const fields = kind === 'metadata' ? versionFields(record.details) : undefined
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
            ? { kind: 'delete' as const, id: record.id }
            : { kind: 'update' as const, id: record.id, details: fields!.value() }
        const next = await options.api.change(index.history, action)
        if (!next || !alive) return
        index = next
        options.changed(next)
        if (kind === 'delete') {
          busy = false
          destroy()
          return
        }
        record = next.versions.find((v) => v.id === record.id)!
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

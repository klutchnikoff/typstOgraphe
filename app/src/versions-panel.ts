import { navigationGroup } from './keyboard-navigation'
import { versionDialog } from './version-dialog'
import { versionFields } from './version-fields'
import { formDialog } from './form-dialog'
import type { VersionAPI, VersionCatalogue } from './version-types'
import type { Catalogue } from './shell'
import { tr } from './i18n'
import { icon } from './icons'

export function versionsPanel(
  root: HTMLElement,
  api: VersionAPI,
  catalogue: Catalogue,
  failed: (e: unknown) => void,
) {
  let alive = true,
    request = 0,
    index: VersionCatalogue | undefined
  let current: ReturnType<typeof versionDialog> | undefined
  let creation: ReturnType<typeof formDialog> | undefined
  const content = document.createElement('div'),
    footer = document.createElement('div'),
    create = document.createElement('button')
  content.className = 'versions-groups'
  footer.className = 'versions-panel-footer'
  create.disabled = true
  create.type = 'button'
  create.className = 'primary'
  create.innerHTML = icon('plus')
  create.append(document.createTextNode(tr('Créer une version')))
  footer.append(create)
  root.replaceChildren(content, footer)
  root.classList.add('versions-panel')
  function render(value: VersionCatalogue) {
    index = value
    const focused = content.contains(document.activeElement)
      ? (document.activeElement as HTMLElement).dataset.version
      : undefined
    content.replaceChildren()
    const count = document.createElement('p')
    count.className = 'versions-quota'
    count.textContent = `${value.versions.length} / ${value.limit}`
    content.append(count)
    create.disabled = value.versions.length >= value.limit
    if (!value.versions.length) {
      const empty = document.createElement('p')
      empty.textContent = tr('Aucune version conservée.')
      content.append(empty)
    }
    for (const [reason, label] of [['manual', 'Versions manuelles']]) {
      const rows = value.versions
        .filter((v) => v.reason === reason)
        .sort((a, b) => b.created_at.localeCompare(a.created_at))
      if (!rows.length) continue
      const group = document.createElement('details'),
        summary = document.createElement('summary'),
        list = document.createElement('div')
      group.open = true
      summary.textContent = `${tr(label)} · ${rows.length}`
      navigationGroup(list, { axis: 'vertical' })
      for (const record of rows) {
        const row = document.createElement('button'),
          name = document.createElement('strong'),
          date = document.createElement('time')
        row.type = 'button'
        row.dataset.version = record.id
        name.textContent = record.details.name
        date.dateTime = record.created_at
        date.textContent = new Date(record.created_at).toLocaleString(catalogue.locale)
        row.append(name, date)
        if (record.details.tags.length) {
          const tags = document.createElement('span')
          tags.className = 'version-tags'
          for (const value of record.details.tags) {
            const tag = document.createElement('span')
            tag.textContent = value
            tags.append(tag)
          }
          row.append(tags)
        }
        row.onclick = () => {
          current?.destroy()
          current = versionDialog({
            record,
            index: index!,
            api,
            catalogue,
            changed: render,
            returnFocus: () =>
              content.querySelector<HTMLElement>(`[data-version="${record.id}"]`) || create,
          })
        }
        list.append(row)
      }
      group.append(summary, list)
      content.append(group)
    }
    if (value.versions.length >= value.limit) {
      const quota = document.createElement('p')
      quota.textContent = tr(
        'La limite de versions est atteinte. Supprimez une version pour en créer une nouvelle.',
      )
      content.append(quota)
    }
    if (focused)
      content
        .querySelector<HTMLElement>(`[data-version="${focused}"]`)
        ?.focus({ preventScroll: true })
  }
  async function refresh() {
    const id = ++request
    try {
      const next = await api.list()
      if (alive && id === request) render(next)
    } catch (error) {
      if (alive && id === request) failed(error)
    }
  }
  create.onclick = () => {
    if (!index) return
    let busy = false
    const close = () => {
      if (!busy) {
        creation?.destroy()
        creation = undefined
      }
    }
    creation = formDialog({
      id: 'version-create',
      title: tr('Créer une version'),
      cancel: close,
      returnFocus: () => create,
    })
    const form = creation,
      fields = versionFields()
    form.body.append(fields.element)
    form.action({ label: tr('Annuler'), run: close })
    form.action({
      label: tr('Créer'),
      primary: true,
      run: () => {
        if (busy) return
        busy = true
        const controls = [...form.element.querySelectorAll<HTMLButtonElement>('button')]
        controls.forEach((b) => (b.disabled = true))
        void api
          .change(index!.history, { kind: 'create', details: fields.value() })
          .then((next) => {
            busy = false
            if (next) {
              render(next)
              close()
            }
          })
          .catch((reason) => {
            form.error.textContent = String(reason)
          })
          .finally(() => {
            busy = false
            controls.forEach((b) => (b.disabled = false))
          })
      },
    })
    form.open()
  }
  void refresh()
  return {
    refresh,
    destroy() {
      alive = false
      request++
      current?.destroy(true)
      creation?.destroy()
      root.classList.remove('versions-panel')
    },
  }
}

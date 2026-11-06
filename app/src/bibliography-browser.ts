import type { Browse } from './bibliography-types'
import { navigationGroup, actionRow } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'

/** Compact list presentation and keyboard ownership; no requests or document edits. */
export function bibliographyBrowser(parent: HTMLElement, activate: (id: string) => void) {
  const layout = document.createElement('div'),
    left = document.createElement('aside'),
    right = document.createElement('section'),
    list = document.createElement('div'),
    search = document.createElement('input'),
    count = document.createElement('p'),
    add = document.createElement('button'),
    load = document.createElement('button'),
    file = document.createElement('input')
  layout.className = 'bibliography-layout'
  left.className = 'bibliography-browser'
  right.className = 'bibliography-detail'
  list.className = 'bibliography-list'
  list.id = 'bibliography-list'
  list.role = 'listbox'
  list.setAttribute('aria-label', tr('Notices'))
  search.type = 'search'
  search.placeholder = tr('Rechercher par titre ou auteur…')
  search.setAttribute('aria-label', tr('Rechercher une référence'))
  search.role = 'combobox'
  search.setAttribute('aria-controls', list.id)
  search.setAttribute('aria-expanded', 'true')
  search.dataset.initialFocus = ''
  add.type = load.type = 'button'
  add.innerHTML = icon('plus')
  add.setAttribute('aria-label', tr('Créer une notice'))
  load.textContent = tr('Importer…')
  file.type = 'file'
  file.accept = '.bib,.yaml,.yml'
  file.hidden = true
  file.setAttribute('aria-label', tr('Fichiers bibliographiques'))
  const top = actionRow()
  top.append(load, add)
  left.append(search, count, top, list, file)
  layout.append(left, right)
  parent.append(layout)
  const navigation = navigationGroup(list, {
    axis: 'vertical',
    input: search,
    onActivate: (row) => {
      activate(row.dataset.recordId!)
    },
  })
  return {
    right,
    list,
    search,
    add,
    load,
    file,
    render(result: Browse, selected?: string) {
      count.textContent = tr('{count} / {total} notices', {
        count: result.entries.length,
        total: result.total,
      })
      list.replaceChildren()
      for (const record of result.entries) {
        const row = document.createElement('button'),
          name = document.createElement('span'),
          title = document.createElement('span')
        row.type = 'button'
        row.dataset.recordId = record.id
        row.className = 'bibliography-list-row'
        row.setAttribute('aria-selected', String(record.id === selected))
        name.textContent = `${record.names} · ${record.year ?? '—'}${record.incomplete ? ' ◦' : ''}`
        title.textContent = record.title || record.key
        row.setAttribute(
          'aria-label',
          `${record.names} · ${record.year ?? '—'} · ${record.title || record.key}${record.incomplete ? ` · ${tr('Notice incomplète')}` : ''}`,
        )
        row.append(name, title)
        list.append(row)
      }
      navigation.refresh()
    },
    select(id?: string) {
      for (const row of list.querySelectorAll<HTMLElement>('[data-record-id]'))
        row.setAttribute('aria-selected', String(row.dataset.recordId === id))
    },
    destroy() {
      navigation.destroy()
      layout.remove()
    },
  }
}

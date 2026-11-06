import { sortableCards } from './sortable-cards'
import { labeledField } from './form-field'
import { metadataSection } from './metadata-fields'
import { actionRow } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import type { Field, Person } from './bibliography-types'
const labels: Record<string, string> = {
  title: 'Titre',
  author: 'Auteurs',
  editor: 'Responsables éditoriaux',
  date: 'Date de publication',
  publisher: 'Éditeur',
  name: 'Nom',
  location: 'Lieu de publication',
  organization: 'Organisme',
  genre: 'Nature du document',
  edition: 'Édition ou version',
  volume: 'Volume',
  issue: 'Numéro',
  'page-range': 'Pages',
  doi: 'DOI',
  isbn: 'ISBN',
  issn: 'ISSN',
  arxiv: 'arXiv',
  value: 'URL',
}
export function fieldLabel(path: string) {
  const leaf = path.split('/').at(-1)!
  if (path.includes('/parent/') && leaf === 'title') return tr('Titre de la publication')
  if (path.startsWith('/url/') && leaf === 'date') return tr('Date de consultation')
  if (path.endsWith('/publisher/name')) return tr('Éditeur')
  return tr(labels[leaf] || leaf)
}
export function diagnosticText(code: string) {
  return tr(
    code === 'incomplete'
      ? 'Un champ conseillé est manquant.'
      : code === 'type_mapping'
        ? 'Vérifier le type de cette notice.'
        : code === 'conversion_review'
          ? 'Vérifier les champs convertis.'
          : ['conversion_loss', 'unsupported_field'].includes(code)
            ? 'Une valeur reste uniquement dans la source originale.'
            : 'Vérifier cette notice.',
  )
}
/** Controls are local values. Only the shared Rust codec produces native field patches. */
export function bibliographyFields(
  parent: HTMLElement,
  fields: Field[],
  values: Record<string, unknown>,
  changed: () => void,
) {
  const cleanups: (() => void)[] = []
  const sections = new Map<string, HTMLElement>()
  for (const field of fields) {
    if (!(field.path in values)) values[field.path] = structuredClone(field.value)
    let section = sections.get(field.group)
    if (!section) {
      const frame = metadataSection({
        id: field.group,
        label: tr(
          field.group === 'work'
            ? 'Notice'
            : field.group === 'publication'
              ? 'Publication'
              : 'Identifiants et liens',
        ),
        layout: 'editorial',
      })
      section = frame.content
      sections.set(field.group, section)
      if (field.group === 'identifiers') {
        const details = document.createElement('details'),
          summary = document.createElement('summary')
        summary.textContent = tr('Identifiants et liens')
        frame.root.querySelector('legend')?.remove()
        details.append(summary, frame.root)
        parent.append(details)
      } else parent.append(frame.root)
    }
    const id = `bibliography-field-${field.path.replaceAll('/', '-')}`
    if (field.kind === 'people') {
      const frame = document.createElement('fieldset'),
        legend = document.createElement('legend'),
        rows = document.createElement('div'),
        add = document.createElement('button')
      frame.className = 'bibliography-people'
      legend.textContent = fieldLabel(field.path)
      add.type = 'button'
      add.innerHTML = icon('plus')
      add.setAttribute('aria-label', tr('Ajouter une personne'))
      const list = () => values[field.path] as Person[]
      const render = () => {
        rows.replaceChildren()
        for (const person of list()) {
          const row = document.createElement('div')
          row.className = 'bibliography-person collection-card'
          row.dataset.recordId = person.id
          const handle = document.createElement('button')
          handle.type = 'button'
          handle.className = 'collection-card-handle'
          handle.innerHTML = icon('grip-vertical')
          handle.setAttribute(
            'aria-label',
            tr('Réordonner · {name}', { name: person.name || fieldLabel(field.path) }),
          )
          row.append(handle)
          row.dataset.personId = person.id
          for (const [key, label] of [
            ['name', 'Nom ou organisme'],
            ['given', 'Prénom'],
          ] as const) {
            const input = document.createElement('input')
            input.id = `${id}-${person.id}-${key}`
            input.value = person[key]
            input.oninput = () => {
              person[key] = input.value
              changed()
            }
            row.append(labeledField(tr(label), input))
          }
          const remove = document.createElement('button')
          remove.type = 'button'
          remove.innerHTML = icon('trash-2')
          remove.setAttribute('aria-label', tr('Retirer cette personne'))
          remove.onclick = () => {
            values[field.path] = list().filter((p) => p !== person)
            render()
            sorting.refresh()
            changed()
            add.focus()
          }
          row.append(remove)
          rows.append(row)
        }
      }
      add.onclick = () => {
        const person = { id: `new-${crypto.randomUUID()}`, name: '', given: '' }
        list().push(person)
        render()
        sorting.refresh()
        changed()
        rows.lastElementChild?.querySelector('input')?.focus()
      }
      frame.append(legend, rows, actionRow())
      frame.lastElementChild!.append(add)
      section.append(frame)
      render()
      const sorting = sortableCards(rows, {
        order: () => list().map((p) => p.id),
        async move(id, before) {
          const person = list().find((p) => p.id === id)!
          const next = list().filter((p) => p.id !== id)
          next.splice(before ? next.findIndex((p) => p.id === before) : next.length, 0, person)
          values[field.path] = next
          changed()
        },
        error: () => {},
      })
      sorting.refresh()
      cleanups.push(() => sorting.destroy())
    } else {
      const input = document.createElement('input')
      input.id = id
      input.value = String(values[field.path] ?? '')
      input.oninput = () => {
        values[field.path] = input.value
        changed()
      }
      if (field.path === '/title') input.dataset.initialFocus = ''
      if (field.kind === 'date') input.placeholder = '2026 / 2026-11-06'
      section.append(labeledField(fieldLabel(field.path), input))
    }
  }
  return () => cleanups.forEach((cleanup) => cleanup())
}

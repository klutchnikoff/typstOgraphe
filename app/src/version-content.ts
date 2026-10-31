import { formatCitationLabel } from './citation-index'
/** Static DOM projection: no editor, Yjs binding, widget actions or live resources. */
import { getSchema, type JSONContent } from '@tiptap/core'
import { DOMSerializer } from '@tiptap/pm/model'
import { richZoneExtensions } from './rich-zone'
import type { VersionContent } from './version-types'
import type { Catalogue } from './shell'
import { tr } from './i18n'

export function versionContent(content: VersionContent, catalogue: Catalogue) {
  const urls: string[] = []
  const resources = new Map(
    content.resources.map((resource) => {
      const bytes = Uint8Array.from(atob(resource.data), (c) => c.charCodeAt(0))
      const extension = resource.path.split('.').pop()
      const type =
        extension === 'svg' ? 'image/svg+xml' : `image/${extension === 'jpg' ? 'jpeg' : extension}`
      const url = URL.createObjectURL(new Blob([bytes], { type }))
      urls.push(url)
      return [resource.path, url]
    }),
  )
  const schema = getSchema(
    richZoneExtensions('document', { imageURL: (path) => resources.get(path) || '' }),
  )
  const element = document.createElement('article')
  element.className = 'version-document'
  element.setAttribute('aria-label', tr('Contenu de la version'))
  let note = 0
  function rich(value: JSONContent) {
    const body = document.createElement('div')
    body.className = 'version-prose'
    body.append(
      DOMSerializer.fromSchema(schema).serializeFragment(schema.nodeFromJSON(value).content),
    )
    for (const badge of body.querySelectorAll<HTMLElement>('[data-footnote]')) {
      badge.textContent = `${tr('Note')} ${++note}`
      badge.title = badge.dataset.footnote || ''
    }
    for (const badge of body.querySelectorAll<HTMLElement>('[data-citation]')) {
      badge.classList.add('citation-ref')
      badge.textContent = formatCitationLabel(
        content.references || [],
        JSON.parse(badge.dataset.citation || '[]'),
        badge.dataset.form === 'prose' ? 'prose' : 'normal',
      )
    }
    body.addEventListener('click', (event) => {
      if ((event.target as HTMLElement).closest('a')) event.preventDefault()
    })
    return body
  }
  const kind = catalogue.types.find((type) => type.id === content.state.settings.metadata.type)
  const information = document.createElement('details')
  const summary = document.createElement('summary')
  summary.textContent = tr('Informations du document')
  information.append(summary)
  const fields = content.state.settings.metadata.fields
  function field(label: string, value: unknown) {
    if (
      value === null ||
      value === undefined ||
      value === '' ||
      (Array.isArray(value) && !value.length)
    )
      return
    const row = document.createElement('div'),
      title = document.createElement('strong')
    row.className = 'version-field'
    title.textContent = label
    row.append(title)
    if (typeof value === 'object' && value && 'type' in value && value.type === 'doc')
      row.append(rich(value as JSONContent))
    else {
      const text = document.createElement('span')
      text.textContent =
        typeof value === 'boolean'
          ? tr(value ? 'Oui' : 'Non')
          : Array.isArray(value)
            ? value.map((v) => (typeof v === 'string' ? v : '')).join(', ')
            : String(value)
      row.append(text)
    }
    information.append(row)
  }
  field(tr('Type de document'), kind?.label || content.state.settings.metadata.type)
  field(
    tr('Modèle'),
    kind?.models.find((model) => model.id === content.state.settings.modelId)?.label ||
      tr('Modèle intégré'),
  )
  for (const definition of kind?.fields || []) field(definition.label, fields[definition.parameter])
  const collections = kind?.structured?.collections || []
  for (const collection of collections) {
    const records = fields[collection.parameter]
    if (!Array.isArray(records)) continue
    const heading = document.createElement('h3')
    heading.textContent = collection.label
    information.append(heading)
    for (const record of records)
      for (const definition of collection.fields) {
        if (definition.kind === 'references') continue
        field(definition.label, record[definition.key])
      }
  }
  element.append(information, rich(content.state.document))
  return {
    element,
    destroy() {
      urls.forEach((url) => URL.revokeObjectURL(url))
    },
  }
}

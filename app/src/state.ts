import { ySyncPluginKey } from '@tiptap/y-tiptap'
import type { Provider } from './provider'
import type { Catalogue, DocumentType } from './shell'
export interface Settings {
  metadata: { type: string; fields: Record<string, unknown> & { options: Record<string, unknown> } }
  template: string
  modelId: string
}
let catalogue: Catalogue
export function initializeSettings(provider: Provider, c: Catalogue, fallback?: Settings) {
  catalogue = c
  const map = provider.doc.getMap('settings')
  if (!map.has('document')) {
    const kind = c.types.find((t) => t.canCreate)!
    map.set(
      'document',
      fallback || {
        metadata: kind.metadata,
        template: kind.models[0].source,
        modelId: kind.models[0].id,
      },
    )
  }
}
export function documentType(provider: Provider): DocumentType {
  const settings = settingsFor(provider)
  return catalogue.types.find((t) => t.id === settings.metadata.type)!
}
export function settingsFor(provider: Provider): Settings {
  const map = provider.doc.getMap('settings'),
    base = structuredClone(map.get('document')) as Settings
  if (!base) return base
  const kind = catalogue.types.find((t) => t.id === base.metadata.type)!
  for (const field of kind.fields)
    if (map.has(field.field)) base.metadata.fields[field.parameter] = map.get(field.field)
  for (const key of ['paper', 'language', 'landscape', 'bullet_marker', 'spelling_ignored'])
    if (map.has(`options.${key}`)) base.metadata.fields.options[key] = map.get(`options.${key}`)
  if (map.has('signature')) base.metadata.fields.signature = map.get('signature')
  return base
}
export function setSetting(provider: Provider, key: string, value: unknown) {
  provider.doc.transact(() => provider.doc.getMap('settings').set(key, value), ySyncPluginKey)
}
export function setDocument(provider: Provider, settings: Settings) {
  provider.doc.transact(() => {
    const map = provider.doc.getMap('settings')
    map.clear()
    map.set('document', settings)
    map.set('identity', crypto.randomUUID())
  }, ySyncPluginKey)
}

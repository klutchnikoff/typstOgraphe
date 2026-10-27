import * as Y from 'yjs'
import { getSchema } from '@tiptap/core'
import {
  prosemirrorJSONToYXmlFragment,
  yXmlFragmentToProsemirrorJSON,
  ySyncPluginKey,
} from '@tiptap/y-tiptap'
import { richZoneExtensions, richFieldProfile } from './rich-zone'
import type { Provider } from './provider'

export interface RecordField {
  key: string
  label: string
  kind:
    'text' | 'multiline' | 'boolean' | 'references' | 'language' | 'tokens' | 'rich' | 'rich_inline'
  default: unknown
  references?: string
}
export interface RecordCollection {
  field: string
  parameter: string
  label: string
  itemLabel: string
  title_key: string
  language_key?: string
  fields: RecordField[]
}
export interface MetadataPanel {
  id: string
  collection: string
  fields?: string[]
}
export type StructuredMetadata = { collections: RecordCollection[]; panels?: MetadataPanel[] }
export type RecordStore = Pick<Provider, 'doc'>
export type RecordData = Record<string, unknown> & { id: string }
export type SharedRecord = Y.Map<unknown>

function collections(provider: RecordStore) {
  return provider.doc.getMap('settings').get('collections') as
    Y.Map<Y.Map<SharedRecord>> | undefined
}
export function recordMap(provider: RecordStore, definition: RecordCollection) {
  return collections(provider)!.get(definition.parameter)!
}
function order(provider: RecordStore, definition: RecordCollection) {
  return provider.doc.getMap('settings').get(`order:${definition.parameter}`) as Y.Array<string>
}
export function recordIds(provider: RecordStore, definition: RecordCollection) {
  const map = recordMap(provider, definition)
  return [...new Set(order(provider, definition).toArray())].filter((id) => map.has(id))
}
export function readRecord(record: SharedRecord, definition: RecordCollection): RecordData {
  return Object.fromEntries([
    ['id', record.get('id')],
    ...definition.fields.map((field) => {
      const value = record.get(field.key)
      return [
        field.key,
        value instanceof Y.XmlFragment
          ? yXmlFragmentToProsemirrorJSON(value)
          : value instanceof Y.Text
            ? value.toString()
            : (value ?? field.default),
      ]
    }),
  ]) as RecordData
}
function fillRecord(record: SharedRecord, definition: RecordCollection, value: RecordData) {
  record.set('id', value.id)
  for (const field of definition.fields) {
    const current = value[field.key] ?? field.default
    if (field.kind === 'rich' || field.kind === 'rich_inline') {
      const fragment = new Y.XmlFragment()
      record.set(field.key, fragment)
      prosemirrorJSONToYXmlFragment(
        getSchema(richZoneExtensions(richFieldProfile(field.kind))),
        current,
        fragment,
      )
    } else if (field.kind === 'text' || field.kind === 'multiline') {
      record.set(field.key, new Y.Text(String(current)))
    } else record.set(field.key, structuredClone(current))
  }
  return record
}
function insert(
  provider: RecordStore,
  definition: RecordCollection,
  value: RecordData,
  after?: string,
) {
  const record = new Y.Map<unknown>()
  recordMap(provider, definition).set(value.id, record)
  fillRecord(record, definition, value)
  const list = order(provider, definition)
  const index = after ? list.toArray().indexOf(after) : -1
  list.insert(index < 0 ? list.length : index + 1, [value.id])
  return record
}

/** Unpublished additions never enter collaboration, snapshots or the document undo history. */
export function createRecordDraft(
  definition: RecordCollection,
  overrides: Record<string, unknown> = {},
) {
  const doc = new Y.Doc()
  const record = doc.getMap('record')
  doc.transact(() =>
    fillRecord(record, definition, { id: crypto.randomUUID(), ...overrides } as RecordData),
  )
  return { doc, record, value: () => readRecord(record, definition), destroy: () => doc.destroy() }
}
export function initializeRecords(
  provider: RecordStore,
  schema: StructuredMetadata | null | undefined,
  values: Record<string, unknown>,
) {
  if (!schema || collections(provider)) return
  provider.doc.transact(() => {
    const root = new Y.Map<Y.Map<SharedRecord>>()
    provider.doc.getMap('settings').set('collections', root)
    for (const definition of schema.collections) {
      root.set(definition.parameter, new Y.Map<SharedRecord>())
      provider.doc.getMap('settings').set(`order:${definition.parameter}`, new Y.Array<string>())
    }
    for (const definition of schema.collections)
      for (const value of (values[definition.parameter] as RecordData[]) || [])
        insert(provider, definition, value)
  }, 'metadata-initialize')
}
export function snapshotRecords(
  provider: RecordStore,
  schema: StructuredMetadata,
  values: Record<string, unknown>,
) {
  if (!collections(provider)) return
  for (const definition of schema.collections)
    values[definition.parameter] = recordIds(provider, definition).map((id) =>
      readRecord(recordMap(provider, definition).get(id)!, definition),
    )
}
export function addRecord(
  provider: RecordStore,
  definition: RecordCollection,
  overrides: Record<string, unknown> = {},
  after?: string,
) {
  let record!: SharedRecord
  provider.doc.transact(() => {
    record = insert(
      provider,
      definition,
      { id: crypto.randomUUID(), ...overrides } as RecordData,
      after,
    )
  }, ySyncPluginKey)
  return record
}
export function setRecordField(
  provider: RecordStore,
  record: SharedRecord,
  key: string,
  value: unknown,
) {
  provider.doc.transact(() => {
    const previous = record.get(key)
    if (previous instanceof Y.Text) {
      const old = Array.from(previous.toString()),
        next = Array.from(String(value))
      let prefix = 0,
        suffix = 0
      while (prefix < old.length && prefix < next.length && old[prefix] === next[prefix]) prefix++
      while (
        suffix < old.length - prefix &&
        suffix < next.length - prefix &&
        old[old.length - 1 - suffix] === next[next.length - 1 - suffix]
      )
        suffix++
      const start = old.slice(0, prefix).join('').length
      const removed = old.slice(prefix, old.length - suffix).join('').length
      const inserted = next.slice(prefix, next.length - suffix).join('')
      if (removed) previous.delete(start, removed)
      if (inserted) previous.insert(start, inserted)
    } else record.set(key, value)
  }, ySyncPluginKey)
}
export function moveRecord(
  provider: RecordStore,
  definition: RecordCollection,
  id: string,
  delta: number,
) {
  const list = order(provider, definition),
    ids = list.toArray(),
    index = ids.indexOf(id),
    next = index + delta
  if (index < 0 || next < 0 || next >= ids.length) return
  provider.doc.transact(() => {
    list.delete(index, 1)
    list.insert(next, [id])
  }, ySyncPluginKey)
}
export function removeRecord(
  provider: RecordStore,
  schema: StructuredMetadata,
  definition: RecordCollection,
  id: string,
  detach = false,
) {
  const linked = schema.collections.flatMap((collection) =>
    collection.fields
      .filter((field) => field.references === definition.parameter)
      .flatMap((field) =>
        recordIds(provider, collection).flatMap((key) => {
          const record = recordMap(provider, collection).get(key)!
          return ((record.get(field.key) as string[]) || []).includes(id) ? [{ record, field }] : []
        }),
      ),
  )
  if (linked.length && !detach) return false
  provider.doc.transact(() => {
    for (const { record, field } of linked)
      record.set(
        field.key,
        (record.get(field.key) as string[]).filter((key) => key !== id),
      )
    const list = order(provider, definition)
    for (let index = list.length - 1; index >= 0; index--)
      if (list.get(index) === id) list.delete(index, 1)
    recordMap(provider, definition).delete(id)
  }, ySyncPluginKey)
  return true
}
export function ensurePrimaryRecords(
  provider: RecordStore,
  schema: StructuredMetadata | null | undefined,
  language: string,
) {
  for (const definition of schema?.collections || []) {
    const key = definition.language_key
    if (
      key &&
      !recordIds(provider, definition).some(
        (id) => recordMap(provider, definition).get(id)!.get(key) === language,
      )
    )
      addRecord(provider, definition, { [key]: language })
  }
}

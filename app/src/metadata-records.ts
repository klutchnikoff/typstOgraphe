import { documentContent } from './shared-inline-marks'
import * as Y from 'yjs'
import { getSchema } from '@tiptap/core'
import {
  prosemirrorJSONToYXmlFragment,
  yXmlFragmentToProsemirrorJSON,
  ySyncPluginKey,
} from '@tiptap/y-tiptap'
import { richZoneExtensions, richFieldProfile } from './rich-zone'
import type { Provider } from './provider'
import { stableJSON } from './provider'
import type { StructuredCommand } from './structured-commands'

export interface RecordField {
  key: string
  label: string
  kind:
    | 'text'
    | 'multiline'
    | 'boolean'
    | 'references'
    | 'language'
    | 'tokens'
    | 'rich'
    | 'rich_inline'
    | 'source'
  default: unknown
  references?: string
}
export interface RecordCollection {
  field: string
  parameter: string
  label: string
  itemLabel: string
  title_key: string
  summary?: string[]
  language_key?: string
  fields: RecordField[]
}
export interface MetadataPanel {
  id: string
  collection: string
  fields?: string[]
  editing?: 'dialog'
}
export interface BibliographyDescriptor {
  tab: string
  files: string
  field: string
  style: string
  style_field: string
}
export type StructuredMetadata = {
  collections: RecordCollection[]
  panels?: MetadataPanel[]
  bibliography?: BibliographyDescriptor
  outline?: { parameter: string; field: string }
}
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
          ? documentContent(yXmlFragmentToProsemirrorJSON(value))
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

/** Unpublished drafts never enter collaboration, snapshots or document undo history. */
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

export type RecordEditError = 'changed' | 'missing-reference' | 'duplicate-language'

/** A private draft produces a typed plan; only the Rust owner may publish it.
 * Preserve unrelated peer edits and refuse changes to this record or its identity.
 */
export function prepareRecordEdit(
  provider: RecordStore,
  schema: StructuredMetadata,
  definition: RecordCollection,
  options: { id?: string; after?: string; defaults?: Record<string, unknown> } = {},
) {
  const map = recordMap(provider, definition)
  const identity = provider.doc.getMap('settings').get('identity')
  const original = options.id ? map.get(options.id) : undefined
  const before = original ? readRecord(original, definition) : undefined
  const fingerprint = stableJSON(before)
  const draft = createRecordDraft(definition, before || options.defaults)
  const validate = (): RecordEditError | undefined => {
    if (
      provider.doc.getMap('settings').get('identity') !== identity ||
      collections(provider)?.get(definition.parameter) !== map ||
      (options.id &&
        (!original ||
          map.get(options.id) !== original ||
          stableJSON(readRecord(original, definition)) !== fingerprint))
    )
      return 'changed'
    const value = draft.value()
    const lang = definition.language_key
    if (
      lang &&
      recordIds(provider, definition).some(
        (id) => id !== options.id && map.get(id)!.get(lang) === value[lang],
      )
    )
      return 'duplicate-language'
    for (const field of definition.fields)
      if (field.references) {
        const target = schema.collections.find((item) => item.parameter === field.references)!
        const ids = recordIds(provider, target)
        if ((value[field.key] as string[]).some((id) => !ids.includes(id)))
          return 'missing-reference'
      }
  }
  return {
    ...draft,
    plan() {
      const error = validate()
      if (error) return { error }
      const value = draft.value(),
        commands: StructuredCommand[] = []
      if (!original) {
        const { id, ...values } = value
        commands.push({
          id: 'record.add',
          params: {
            collection: definition.parameter,
            id,
            values,
            ...(options.after ? { after: options.after } : {}),
          },
        })
      } else
        for (const field of definition.fields) {
          if (stableJSON(before![field.key]) === stableJSON(value[field.key])) continue
          if (field.kind === 'rich' || field.kind === 'rich_inline') {
            commands.push({
              id: 'block.splice',
              params: {
                parent: {
                  root: {
                    kind: 'record',
                    collection: definition.parameter,
                    id: value.id,
                    field: field.key,
                  },
                  path: [],
                },
                index: 0,
                delete_count: (before![field.key] as { content: unknown[] }).content.length,
                nodes: (value[field.key] as { content: unknown[] }).content,
              },
            })
          } else
            commands.push({
              id: 'metadata.set',
              params: {
                target: {
                  kind: 'record',
                  collection: definition.parameter,
                  id: value.id,
                  field: field.key,
                },
                value: value[field.key],
              },
            })
        }
      const frozen = stableJSON(value)
      return {
        id: value.id,
        commands,
        current: () => !validate() && stableJSON(draft.value()) === frozen,
      }
    },
  }
}
export function initializeRecords(
  provider: RecordStore,
  schema: StructuredMetadata | null | undefined,
  values: Record<string, unknown>,
) {
  if (!schema) return
  provider.doc.transact(() => {
    let root = collections(provider)
    if (!root) {
      root = new Y.Map<Y.Map<SharedRecord>>()
      provider.doc.getMap('settings').set('collections', root)
    }
    for (const definition of schema.collections) {
      if (root.has(definition.parameter)) continue
      root.set(definition.parameter, new Y.Map<SharedRecord>())
      provider.doc.getMap('settings').set(`order:${definition.parameter}`, new Y.Array<string>())
      for (const value of (values[definition.parameter] as RecordData[]) || [])
        insert(provider, definition, value)
    }
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
    } else if (previous instanceof Y.XmlFragment) {
      prosemirrorJSONToYXmlFragment(getSchema(richZoneExtensions('rich-text')), value, previous)
    } else record.set(key, value)
  }, ySyncPluginKey)
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

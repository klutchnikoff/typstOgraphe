import { expect, it } from 'vitest'
import * as Y from 'yjs'
import { ySyncPluginKey } from '@tiptap/y-tiptap'
import type { Provider } from '../src/provider'
import {
  initializeRecords,
  snapshotRecords,
  recordMap,
  recordIds,
  setRecordField,
  addRecord,
  removeRecord,
  moveRecord,
  type StructuredMetadata,
} from '../src/metadata-records'

const rich = (text: string) => ({
  type: 'doc',
  content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
})
const schema: StructuredMetadata = {
  collections: [
    {
      field: 'sample.people',
      parameter: 'people',
      label: 'People',
      itemLabel: 'Person',
      title_key: 'name',
      fields: [
        { key: 'name', label: 'Name', kind: 'text', default: '' },
        {
          key: 'affiliations',
          label: 'Affiliations',
          kind: 'references',
          references: 'affiliations',
          default: [],
        },
        { key: 'summary', label: 'Summary', kind: 'rich', default: rich('Résumé') },
      ],
    },
    {
      field: 'sample.affiliations',
      parameter: 'affiliations',
      label: 'Affiliations',
      itemLabel: 'Affiliation',
      title_key: 'text',
      fields: [{ key: 'text', label: 'Text', kind: 'multiline', default: '' }],
    },
  ],
}
function setup() {
  const doc = new Y.Doc(),
    provider = { doc } as Provider
  initializeRecords(provider, schema, {
    people: [{ id: 'a', name: 'Camille', affiliations: ['f'], summary: rich('Résumé') }],
    affiliations: [{ id: 'f', text: 'Univ Rennes, CNRS, IRMAR - UMR 6625' }],
  })
  return provider
}
function snapshot(provider: Provider) {
  const value: Record<string, unknown> = {}
  snapshotRecords(provider, schema, value)
  return value as { people: Record<string, unknown>[]; affiliations: Record<string, unknown>[] }
}
it('independent field and text edits converge, including Unicode and a named rich zone', () => {
  const a = setup(),
    b = { doc: new Y.Doc() } as Provider
  Y.applyUpdate(b.doc, Y.encodeStateAsUpdate(a.doc), 'peer')
  const personA = recordMap(a, schema.collections[0]).get('a')!,
    personB = recordMap(b, schema.collections[0]).get('a')!
  const affiliationB = recordMap(b, schema.collections[1]).get('f')!
  setRecordField(a, personA, 'name', 'Dr Camille')
  setRecordField(b, personB, 'name', 'Camille 👩🏽‍🔬')
  setRecordField(
    b,
    affiliationB,
    'text',
    'Signature officielle inchangée : Univ Rennes, CNRS, IRMAR',
  )
  const fragmentA = personA.get('summary') as Y.XmlFragment,
    fragmentB = personB.get('summary') as Y.XmlFragment
  const textA = (fragmentA.get(0) as Y.XmlElement).get(0) as Y.XmlText
  const textB = (fragmentB.get(0) as Y.XmlElement).get(0) as Y.XmlText
  a.doc.transact(() => textA.insert(0, 'Premier '), ySyncPluginKey)
  b.doc.transact(() => textB.insert(textB.length, ' enrichi'), ySyncPluginKey)
  Y.applyUpdate(a.doc, Y.encodeStateAsUpdate(b.doc), 'peer')
  Y.applyUpdate(b.doc, Y.encodeStateAsUpdate(a.doc), 'peer')
  expect(snapshot(a)).toEqual(snapshot(b))
  expect(snapshot(a).people[0].name).toBe('Dr Camille 👩🏽‍🔬')
  expect(snapshot(a).affiliations[0].text).toBe(
    'Signature officielle inchangée : Univ Rennes, CNRS, IRMAR',
  )
  expect(JSON.stringify(snapshot(a).people[0].summary)).toContain('Premier Résumé enrichi')
  a.doc.destroy()
  b.doc.destroy()
})
it('used affiliations require explicit detachment and one undo restores records and links', () => {
  const provider = setup(),
    undo = new Y.UndoManager(provider.doc.getMap('settings'), {
      trackedOrigins: new Set([ySyncPluginKey]),
    })
  expect(removeRecord(provider, schema, schema.collections[1], 'f')).toBe(false)
  expect(snapshot(provider).people[0].affiliations).toEqual(['f'])
  expect(removeRecord(provider, schema, schema.collections[1], 'f', true)).toBe(true)
  expect(snapshot(provider).people[0].affiliations).toEqual([])
  expect(snapshot(provider).affiliations).toEqual([])
  undo.undo()
  expect(snapshot(provider).people[0].affiliations).toEqual(['f'])
  expect(snapshot(provider).affiliations[0].text).toContain('IRMAR')
  provider.doc.destroy()
})
it('reordering is separate from stable identities and values; creation and deletion are undoable', () => {
  const provider = setup(),
    undo = new Y.UndoManager(provider.doc.getMap('settings'), {
      trackedOrigins: new Set([ySyncPluginKey]),
    })
  const second = addRecord(provider, schema.collections[0], { name: 'Camille' }),
    id = second.get('id') as string
  undo.stopCapturing()
  moveRecord(provider, schema.collections[0], id, -1)
  expect(recordIds(provider, schema.collections[0])).toEqual([id, 'a'])
  expect(snapshot(provider).people.map((p) => p.name)).toEqual(['Camille', 'Camille'])
  undo.undo()
  expect(recordIds(provider, schema.collections[0])).toEqual(['a', id])
  undo.undo()
  expect(recordIds(provider, schema.collections[0])).toEqual(['a'])
  provider.doc.destroy()
})

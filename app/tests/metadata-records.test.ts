import { expect, it } from 'vitest'
import * as Y from 'yjs'
import { ySyncPluginKey } from '@tiptap/y-tiptap'
import type { Provider } from '../src/provider'
import { removeRecordCommands, moveRecordCommands } from '../src/metadata-record-commands'
import {
  initializeRecords,
  snapshotRecords,
  recordMap,
  recordIds,
  setRecordField,
  addRecord,
  createRecordDraft,
  prepareRecordEdit,
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
it('used affiliations require explicit detachment in the atomic deletion plan', () => {
  const provider = setup(),
    before = snapshot(provider)
  expect(removeRecordCommands(provider, schema, schema.collections[1], 'f')).toBeNull()
  const commands = removeRecordCommands(provider, schema, schema.collections[1], 'f', true)!
  expect(commands.map((command) => command.id)).toEqual(['metadata.set', 'record.remove'])
  expect(commands[0].params).toMatchObject({
    target: { kind: 'record', collection: 'people', id: 'a', field: 'affiliations' },
    value: [],
  })
  expect(snapshot(provider)).toEqual(before)
  provider.doc.destroy()
})
it('reordering translates visible positions to stable ID anchors without modifying records', () => {
  const provider = setup()
  const second = addRecord(provider, schema.collections[0], { name: 'Camille' }),
    id = second.get('id') as string
  expect(moveRecordCommands(provider, schema.collections[0], id, -1)).toEqual([
    { id: 'record.move', params: { collection: 'people', id, before: 'a' } },
  ])
  expect(moveRecordCommands(provider, schema.collections[0], 'a', 1)).toEqual([
    { id: 'record.move', params: { collection: 'people', id: 'a' } },
  ])
  expect(recordIds(provider, schema.collections[0])).toEqual(['a', id])
  provider.doc.destroy()
})

it('pending additions are private; cancellation preserves peer changes and document undo', () => {
  const provider = setup()
  const undo = new Y.UndoManager(provider.doc.getMap('settings'), {
    trackedOrigins: new Set([ySyncPluginKey]),
  })
  const before = snapshot(provider)
  const draft = createRecordDraft(schema.collections[0])
  setRecordField(draft, draft.record, 'name', 'Nouvel auteur 👩🏽‍🔬')
  setRecordField(draft, draft.record, 'affiliations', ['f'])
  expect(snapshot(provider)).toEqual(before)
  expect(undo.undoStack).toHaveLength(0)
  const peer = { doc: new Y.Doc() } as Provider
  Y.applyUpdate(peer.doc, Y.encodeStateAsUpdate(provider.doc), 'peer')
  setRecordField(peer, recordMap(peer, schema.collections[0]).get('a')!, 'name', 'Collègue')
  Y.applyUpdate(provider.doc, Y.encodeStateAsUpdate(peer.doc), 'peer')
  draft.destroy()
  expect(snapshot(provider).people).toHaveLength(1)
  expect(snapshot(provider).people[0].name).toBe('Collègue')
  expect(undo.undoStack).toHaveLength(0)
  provider.doc.destroy()
  peer.doc.destroy()
})
it('confirming a filled draft inserts after its anchor in a single undoable transaction', () => {
  const provider = setup()
  const undo = new Y.UndoManager(provider.doc.getMap('settings'), {
    trackedOrigins: new Set([ySyncPluginKey]),
  })
  const last = addRecord(provider, schema.collections[0], { name: 'Dernier' })
  undo.clear()
  const draft = createRecordDraft(schema.collections[0])
  setRecordField(draft, draft.record, 'name', 'Auteur intermédiaire')
  setRecordField(draft, draft.record, 'affiliations', ['f'])
  const id = draft.value().id
  addRecord(provider, schema.collections[0], draft.value(), 'a')
  draft.destroy()
  expect(recordIds(provider, schema.collections[0])).toEqual(['a', id, last.get('id')])
  expect(snapshot(provider).people[1].name).toBe('Auteur intermédiaire')
  expect(snapshot(provider).people[1].affiliations).toEqual(['f'])
  expect(JSON.stringify(snapshot(provider).people[1].summary)).toContain('Résumé')
  expect(undo.undoStack).toHaveLength(1)
  undo.undo()
  expect(recordIds(provider, schema.collections[0])).toEqual(['a', last.get('id')])
  undo.redo()
  expect(snapshot(provider).people[1].name).toBe('Auteur intermédiaire')
  provider.doc.destroy()
})

it('editing a private draft builds a typed plan without publishing any field', () => {
  const provider = setup(),
    definition = schema.collections[0],
    before = snapshot(provider),
    edit = prepareRecordEdit(provider, schema, definition, { id: 'a' })
  let updates = 0
  provider.doc.on('update', () => updates++)
  setRecordField(edit, edit.record, 'name', 'Camille 👩🏽‍🔬')
  setRecordField(edit, edit.record, 'affiliations', [])
  setRecordField(edit, edit.record, 'summary', rich('Résumé enrichi'))
  const plan = edit.plan()
  expect(plan.id).toBe('a')
  expect(plan.commands?.map((command) => command.id)).toEqual([
    'metadata.set',
    'metadata.set',
    'block.splice',
  ])
  expect(plan.current?.()).toBe(true)
  expect(snapshot(provider)).toEqual(before)
  expect(updates).toBe(0)
  setRecordField(edit, edit.record, 'name', 'Autre saisie')
  expect(plan.current?.()).toBe(false)
  edit.destroy()
  provider.doc.destroy()
})

it.each(['change', 'delete', 'replace', 'document'])(
  'a concurrent %s cannot be overwritten by an edit draft',
  (action) => {
    const provider = setup(),
      definition = schema.collections[0],
      edit = prepareRecordEdit(provider, schema, definition, { id: 'a' })
    setRecordField(edit, edit.record, 'name', 'Saisie privée')
    if (action === 'change')
      setRecordField(provider, recordMap(provider, definition).get('a')!, 'name', 'Collègue')
    else if (action === 'document')
      provider.doc.getMap('settings').set('identity', 'another-document')
    else {
      recordMap(provider, definition).delete('a')
      if (action === 'replace')
        addRecord(provider, definition, {
          id: 'a',
          name: 'Camille',
          affiliations: ['f'],
          summary: rich('Résumé'),
        })
    }
    const before = snapshot(provider)
    expect(edit.plan()).toEqual({ error: 'changed' })
    expect(edit.value().name).toBe('Saisie privée')
    expect(snapshot(provider)).toEqual(before)
    edit.destroy()
    provider.doc.destroy()
  },
)

it('independent edits refresh referenced records and remain compatible with a private draft', () => {
  const provider = setup(),
    edit = prepareRecordEdit(provider, schema, schema.collections[0], { id: 'a' })
  setRecordField(edit, edit.record, 'name', 'Camille Martin')
  setRecordField(
    provider,
    recordMap(provider, schema.collections[1]).get('f')!,
    'text',
    'Signature actualisée',
  )
  expect(edit.plan()).toMatchObject({ id: 'a' })
  expect(snapshot(provider).affiliations[0].text).toBe('Signature actualisée')
  edit.destroy()
  provider.doc.destroy()
})

it('a deleted reference prevents publication without discarding the draft; clearing it permits confirmation', () => {
  const provider = setup(),
    edit = prepareRecordEdit(provider, schema, schema.collections[0])
  setRecordField(edit, edit.record, 'name', 'Nouvel auteur')
  setRecordField(edit, edit.record, 'affiliations', ['f'])
  recordMap(provider, schema.collections[1]).delete('f')
  setRecordField(provider, recordMap(provider, schema.collections[0]).get('a')!, 'affiliations', [])
  const before = snapshot(provider)
  expect(edit.plan()).toEqual({ error: 'missing-reference' })
  expect(snapshot(provider)).toEqual(before)
  expect(edit.value().name).toBe('Nouvel auteur')
  setRecordField(edit, edit.record, 'affiliations', [])
  expect(edit.plan()).toMatchObject({ id: edit.value().id, commands: [{ id: 'record.add' }] })
  expect(snapshot(provider)).toEqual(before)
  edit.destroy()
  provider.doc.destroy()
})

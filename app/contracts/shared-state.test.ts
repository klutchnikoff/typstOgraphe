import { documentContent } from '../src/shared-inline-marks'
import { beforeAll, afterAll, afterEach, expect, it } from 'vitest'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createInterface } from 'node:readline'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve, join } from 'node:path'
import { getSchema, type Editor, type JSONContent } from '@tiptap/core'
import { prosemirrorJSONToYXmlFragment, yXmlFragmentToProsemirrorJSON } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { createRichZone, richZoneExtensions } from '../src/rich-zone'
import { initializeSettings, settingsFor, setSetting } from '../src/state'
import { addRecord, setRecordField, prepareRecordEdit, recordMap } from '../src/metadata-records'
import { moveRecordCommands, removeRecordCommands } from '../src/metadata-record-commands'
import type { Provider } from '../src/provider'
import type { Catalogue } from '../src/shell'
import { formattingBatch } from '../src/formatting'
import { inlineTarget, treeTarget, widgetBatch } from '../src/object-commands'
import { replacementPlan } from '../src/text-edits'
import { findMatches } from '../src/proofing'
import { interactionAvailability } from '../src/command-availability'
import { positionAtPath } from '../src/commands'

let child: ChildProcessWithoutNullStreams
let catalogue: Catalogue
let directory: string
const pending: { resolve: (value: any) => void; reject: (error: unknown) => void }[] = []
const docs: Y.Doc[] = []
const editors: Editor[] = []
const schema = getSchema(richZoneExtensions('document'))
function call(command: string, args: unknown) {
  return new Promise<any>((resolve, reject) => {
    pending.push({ resolve, reject })
    child.stdin.write(JSON.stringify({ peer: 'contract-test', command, args }) + '\n')
  })
}
const probe = (args: unknown) => call('diagnostic_shared_state', args)
const bytes = (doc: Y.Doc) => [...Y.encodeStateAsUpdate(doc)]
const store = (doc: Y.Doc) => ({ doc }) as Provider
function doc(update?: number[]) {
  const value = new Y.Doc()
  docs.push(value)
  if (update) Y.applyUpdate(value, new Uint8Array(update))
  return value
}
function fixture(article: boolean): JSONContent {
  const document: JSONContent = {
    type: 'doc',
    content: [
      { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Un titre' }] },
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'A😀é 日本', marks: [{ type: 'bold' }, { type: 'italic' }] },
          {
            type: 'text',
            text: ' lien',
            marks: [{ type: 'link', attrs: { href: 'https://example.org' } }],
          },
          { type: 'footnote', attrs: { text: 'Une note é😀' } },
          ...(article
            ? [
                {
                  type: 'citation',
                  attrs: {
                    items: [{ file_id: 'main', key: 'test2026', supplement: 'p. 12' }],
                    form: 'normal',
                  },
                },
              ]
            : []),
        ],
      },
      {
        type: 'quote',
        content: [
          { type: 'paragraph', content: [{ type: 'text', text: 'Citation' }] },
          { type: 'quoteAttribution', content: [{ type: 'text', text: 'Attribution' }] },
        ],
      },
      {
        type: 'descriptionList',
        content: [
          {
            type: 'descriptionItem',
            content: [
              { type: 'descriptionTerm', content: [{ type: 'text', text: 'Terme' }] },
              {
                type: 'descriptionDefinition',
                content: [{ type: 'text', text: 'Définition' }],
              },
            ],
          },
        ],
      },
      {
        type: 'table',
        content: [
          {
            type: 'tableRow',
            content: [
              {
                type: 'tableCell',
                content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Cellule' }] }],
              },
            ],
          },
        ],
      },
      {
        type: 'figure',
        attrs: { src: 'images/parc.svg', alt: 'Un parc' },
        content: [{ type: 'text', text: 'Légende' }],
      },
      { type: 'paragraph' },
    ],
  }
  document.content = document.content!.map((node) =>
    node.type === 'table'
      ? { type: 'tableFigure', content: [node, { type: 'tableCaption' }] }
      : node,
  )
  return document
}
function seed(type: 'courrier' | 'article') {
  const kind = catalogue.types.find((t) => t.id === type)!
  const settings = structuredClone({
    metadata: kind.metadata,
    template: kind.models[0].source,
    modelId: kind.models[0].id,
  })
  if (type === 'article')
    settings.metadata.fields.references = [
      {
        id: 'main',
        name: 'references.bib',
        contents: '@book{test2026, author={Martin, Camille}, title={Un ouvrage}, year={2026}}',
      },
    ]
  const value = doc()
  const content = schema.nodeFromJSON(fixture(type === 'article')).toJSON()
  prosemirrorJSONToYXmlFragment(schema, content, value.getXmlFragment('default'))
  initializeSettings(store(value), catalogue, settings)
  return value
}
function state(value: Y.Doc) {
  return {
    version: 1,
    document: documentContent(
      schema.nodeFromJSON(yXmlFragmentToProsemirrorJSON(value.getXmlFragment('default'))).toJSON(),
    ),
    settings: settingsFor(store(value)),
  }
}
beforeAll(async () => {
  directory = mkdtempSync(join(tmpdir(), 'tog-shared-contract-'))
  const binary =
    process.env.TOG_SPIKE_BINARY ||
    resolve('../target/debug/typstographe' + (process.platform === 'win32' ? '.exe' : ''))
  child = spawn(binary, ['--stdio', '--session-dir', directory], {
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  let stderr = ''
  child.stderr.on('data', (data) => {
    stderr = (stderr + String(data)).slice(-4000)
  })
  createInterface({ input: child.stdout }).on('line', (line) => {
    const request = pending.shift()
    if (!request) return
    try {
      const message = JSON.parse(line)
      if (message.error) request.reject(message.error)
      else request.resolve(message.result)
    } catch (error) {
      request.reject(error)
    }
  })
  child.on('error', (error) => pending.splice(0).forEach((p) => p.reject(error)))
  child.on('exit', (code) =>
    pending.splice(0).forEach((p) => p.reject(`native driver exited ${code}: ${stderr}`)),
  )
  catalogue = await call('content_catalogue', {})
})
afterEach(() => {
  editors.splice(0).forEach((e) => e.destroy())
  docs.splice(0).forEach((d) => d.destroy())
  document.body.replaceChildren()
})
afterAll(async () => {
  if (child && child.exitCode === null) {
    await new Promise<void>((resolve) => {
      child.once('exit', () => resolve())
      child.stdin.end()
    })
  }
  if (directory) rmSync(directory, { recursive: true, force: true })
})

it('Rust and the immediate editor projection agree on action availability across body, widgets and header', async () => {
  const original = seed('article')
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Availability',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const capabilities = catalogue.entries.filter((c) =>
    catalogue.types.find((t) => t.id === 'article')!.capabilities.includes(c.id),
  )
  for (const path of [
    [0],
    [1],
    [2, 0],
    [2, 1],
    [3, 0, 0],
    [3, 0, 1],
    [4, 0, 0, 0, 0],
    [4, 1],
    [5],
    [6],
  ]) {
    editor.commands.setTextSelection(positionAtPath(editor, path) + 1)
    const target = inlineTarget(editor, editor.state.selection.from, editor.state.selection.to)
    const result = await probe({
      action: 'editing',
      update: bytes(original),
      guard: { document: 'availability', revision: 0 },
      contexts: 1,
      steps: [{ op: 'select', actor: 0, target }],
    })
    const confirmed = result.events[0].contexts[0].availability
    for (const c of capabilities)
      expect(
        interactionAvailability(editor, c.editor!.availability!),
        `${c.id} at ${path}`,
      ).toEqual(confirmed[c.id])
    expect(result.events[0].contexts[0].undo_steps).toBe(0)
    expect(result.guard.revision).toBe(0)
  }
})

it.each(['courrier', 'article'] as const)(
  'reads actual Yjs editor types and preserves %s project identities',
  async (type) => {
    const original = seed(type)
    const expected = state(original)
    original.getMap('__tiptap_position_migrations:root:default').set('kept', true)
    const inspected = await probe({ action: 'inspect', update: bytes(original), expected })
    const restored = doc(inspected.update)
    expect(
      documentContent(
        schema
          .nodeFromJSON(yXmlFragmentToProsemirrorJSON(restored.getXmlFragment('default')))
          .toJSON(),
      ),
    ).toEqual(expected.document)
    expect(restored.getMap('__tiptap_position_migrations:root:default').get('kept')).toBe(true)
    expect([...Y.encodeStateVector(restored)]).toEqual([...Y.encodeStateVector(original)])
    await probe({
      action: 'inspect',
      update: bytes(restored),
      expected: { ...expected, settings: settingsFor(store(restored)) },
    })
  },
)

it('opens Rust-initialized XML, styled text and badges in the actual editor', async () => {
  const original = seed('article')
  const expected = state(original)
  const styleAnnotations = (node: JSONContent) => {
    if (node.type === 'footnote' || node.type === 'citation') node.marks = [{ type: 'italic' }]
    node.content?.forEach(styleAnnotations)
  }
  styleAnnotations(expected.document)
  const initialized = await probe({ action: 'initialize', state: expected })
  const restored = doc(initialized.update)
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Contract',
    language: 'fr',
    binding: { kind: 'document', document: restored },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  expect(documentContent(editor.getJSON())).toEqual(expected.document)
  expect(editor.view.dom.querySelector('.citation-ref')).toBeTruthy()
  expect(editor.view.dom.querySelector('.footnote-ref')).toBeTruthy()
  editor.commands.insertContentAt(1, 'Ajout ')
  await probe({ action: 'inspect', update: bytes(restored), expected: state(restored) })
})

it('projects per-record text, rich abstracts and options from shared metadata', async () => {
  const original = seed('article')
  const definition = catalogue.types.find((t) => t.id === 'article')!.structured!
  const authors = definition.collections.find((c) => c.parameter === 'authors')!
  const author = addRecord(store(original), authors)
  setRecordField(store(original), author, 'name', 'Camille 😀')
  const collections = original.getMap('settings').get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>
  const localizations = collections.get('localizations')!
  const record = [...localizations.values()][0]
  const fragment = record.get('abstract') as Y.XmlFragment
  prosemirrorJSONToYXmlFragment(
    getSchema(richZoneExtensions('rich-text')),
    {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [{ type: 'text', text: 'Résumé 日本', marks: [{ type: 'italic' }] }],
        },
      ],
    },
    fragment,
  )
  setSetting(store(original), 'article.show_outline', true)
  const expected = state(original)
  const result = await probe({ action: 'inspect', update: bytes(original), expected })
  expect(result.state.settings.metadata.fields.authors[0].name).toBe('Camille 😀')
  expect(result.state.settings.metadata.fields.show_outline).toBe(true)
})

it.each(['courrier', 'article'] as const)(
  'preserves inline annotation styles, remote projection and undo in %s',
  async (type) => {
    const original = seed(type)
    const element = document.createElement('div')
    document.body.append(element)
    const editor = createRichZone({
      profile: 'document',
      element,
      label: 'Styled annotation',
      binding: { kind: 'document', document: original },
    })
    editors.push(editor)
    if (!editor.isInitialized)
      await new Promise<void>((resolve) => editor.once('create', () => resolve()))
    editor.chain().selectAll().toggleBold().run()
    await probe({
      action: 'inspect',
      update: bytes(original),
      expected: { ...state(original), document: documentContent(editor.getJSON()) },
    })
    const baseline = state(original)
    const other = doc(bytes(original))
    const otherElement = document.createElement('div')
    document.body.append(otherElement)
    const remote = createRichZone({
      profile: 'document',
      element: otherElement,
      label: 'Remote annotation',
      binding: { kind: 'document', document: other },
    })
    editors.push(remote)
    if (!remote.isInitialized)
      await new Promise<void>((resolve) => remote.once('create', () => resolve()))
    expect(documentContent(remote.getJSON())).toEqual(documentContent(editor.getJSON()))
    expect(editor.commands.undo()).toBe(true)
    Y.applyUpdate(other, Y.encodeStateAsUpdate(original), 'remote')
    expect(documentContent(remote.getJSON())).toEqual(documentContent(editor.getJSON()))
    expect(JSON.stringify(documentContent(remote.getJSON()))).not.toEqual(
      JSON.stringify(baseline.document),
    )
    await probe({
      action: 'inspect',
      update: bytes(other),
      expected: { ...state(other), document: documentContent(remote.getJSON()) },
    })
  },
)

it('publishes a validated differential readable by Yjs, and rejects an obsolete guard', async () => {
  const original = seed('courrier')
  const remote = doc(bytes(original))
  const before = Y.encodeStateVector(remote)
  const paragraph = remote.getXmlFragment('default').get(1) as Y.XmlElement
  ;(paragraph.get(0) as Y.XmlText).insert(3, ' Rust ')
  const updates = [[...Y.encodeStateAsUpdate(remote, before)]]
  const guard = { document: 'contract-project', revision: 0 }
  const outcome = await probe({
    action: 'publish',
    update: bytes(original),
    guard,
    expected: guard,
    updates,
  })
  expect(outcome.guard.revision).toBe(1)
  Y.applyUpdate(original, new Uint8Array(outcome.update))
  await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
  expect(JSON.stringify(outcome.state.document)).toContain('A😀 Rust é')
  await expect(
    probe({
      action: 'publish',
      update: bytes(original),
      guard: outcome.guard,
      expected: guard,
      updates,
    }),
  ).rejects.toContain('stale_revision')
})

it('executes one native multizone batch in the actual editor, preserving identities and one undo step', async () => {
  const original = seed('article')
  const collections = catalogue.types.find((t) => t.id === 'article')!.structured!.collections
  const author = addRecord(
    store(original),
    collections.find((c) => c.parameter === 'authors')!,
    { id: 'camille', name: 'Camille' },
  )
  const authorName = author.get('name')
  const root = original.getXmlFragment('default')
  const paragraph = root.get(1) as Y.XmlElement
  const note = paragraph
    .toArray()
    .find((v) => v instanceof Y.XmlElement && v.nodeName === 'footnote')
  const localizations = (
    original.getMap('settings').get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>
  ).get('localizations')!
  const localized = [...localizations.values()][0]
  const localizedId = localized.get('id') as string
  const abstract = localized.get('abstract')
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Native multizone',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const before = state(original)
  const origin = Symbol('confirmed-native-command')
  const undo = new Y.UndoManager([root, original.getMap('settings')], {
    trackedOrigins: new Set([origin]),
  })
  const guard = { document: 'multizone', revision: 0 }
  const range = (zone: unknown, from: number, to: number) => ({ zone, from, to })
  const commands = [
    {
      id: 'text.insert',
      params: {
        target: range({ kind: 'annotation', path: [1, 2] }, 0, 'Une note é😀'.length),
        text: 'Note revue 日本',
      },
    },
    {
      id: 'text.insert',
      params: {
        target: range({ kind: 'body', path: [4, 0, 0, 0, 0] }, 0, 7),
        text: 'Cellule revue',
      },
    },
    {
      id: 'metadata.set',
      params: {
        target: { kind: 'record', collection: 'authors', id: 'camille', field: 'name' },
        value: 'Camille Martin',
      },
    },
    {
      id: 'text.insert',
      params: {
        target: range(
          {
            kind: 'record',
            collection: 'localizations',
            id: localizedId,
            field: 'abstract',
            path: [0],
          },
          0,
          0,
        ),
        text: 'Résumé natif 😀',
        marks: [{ type: 'italic' }],
      },
    },
    { id: 'text.clear', params: { target: range({ kind: 'body', path: [1] }, 0, 3) } },
    {
      id: 'text.italic',
      params: { target: range({ kind: 'body', path: [1] }, 12, 14), value: 'on' },
    },
  ]
  const outcome = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands,
  })
  expect(outcome.guard.revision).toBe(1)
  Y.applyUpdate(original, new Uint8Array(outcome.update), origin)
  expect(root.get(1)).toBe(paragraph)
  expect(paragraph.toArray()).toContain(note)
  expect(author.get('name')).toBe(authorName)
  expect(localized.get('abstract')).toBe(abstract)
  expect(undo.undoStack).toHaveLength(1)
  expect(editor.view.dom.querySelector('td')?.textContent).toBe('Cellule revue')
  expect(author.get('name')?.toString()).toBe('Camille Martin')
  const atoms = documentContent(editor.getJSON()).content![1].content!.filter((n) =>
    ['footnote', 'citation'].includes(n.type!),
  )
  expect(atoms).toHaveLength(2)
  expect(atoms.every((n) => n.marks?.some((m) => m.type === 'italic'))).toBe(true)
  expect(atoms[0].attrs?.text).toBe('Note revue 日本')
  await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
  undo.undo()
  await probe({ action: 'inspect', update: bytes(original), expected: before })
  undo.redo()
  expect(editor.view.dom.querySelector('td')?.textContent).toBe('Cellule revue')
  undo.destroy()
})

it('native styles remove hashed link attributes and an explicit plain insertion cannot inherit them', async () => {
  const original = seed('courrier')
  const guard = { document: 'style-contract', revision: 0 }
  const range = (from: number, to: number) => ({ zone: { kind: 'body', path: [1] }, from, to })
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: [
      { id: 'text.remove_link', params: { target: range(7, 12) } },
      { id: 'text.insert', params: { target: range(1, 1), text: 'plain' } },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(result.update))
  const content = state(original).document.content![1].content!
  expect(content.some((node) => node.marks?.some((mark) => mark.type === 'link'))).toBe(false)
  expect(content.find((node) => node.text?.includes('plain'))?.marks?.length || 0).toBe(0)
  await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
})

it('applies native structural edits in Tiptap without rebuilding neighbors and undoes the whole batch', async () => {
  const original = seed('article')
  const root = original.getXmlFragment('default')
  const originalHeading = root.get(0)
  const originalParagraph = root.get(1)
  const table = root.get(4)
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Structures',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const before = state(original)
  const origin = Symbol('native-structure')
  const undo = new Y.UndoManager(root, { trackedOrigins: new Set([origin]) })
  const tree = (path: number[]) => ({ root: { kind: 'body' }, path })
  const paragraph = (text: string) => ({ type: 'paragraph', content: [{ type: 'text', text }] })
  const guard = { document: 'structures', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: [
      {
        id: 'block.split',
        params: { target: { zone: { kind: 'body', path: [1] }, from: 3, to: 3 } },
      },
      {
        id: 'block.insert',
        params: {
          parent: tree([]),
          index: 2,
          node: {
            type: 'bulletList',
            content: [{ type: 'listItem', content: [paragraph('Premier item')] }],
          },
        },
      },
      {
        id: 'block.insert',
        params: {
          parent: tree([2]),
          index: 1,
          node: { type: 'listItem', content: [paragraph('Second item')] },
        },
      },
      { id: 'block.heading_3', params: { target: tree([0]) } },
      { id: 'block.remove', params: { target: tree([4]) } },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(result.update), origin)
  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(root.get(0)).toBe(originalHeading)
  expect(root.get(1)).toBe(originalParagraph)
  expect(root.toArray()).toContain(table)
  expect(editor.view.dom.querySelectorAll('li')).toHaveLength(2)
  expect(editor.view.dom.querySelector('h3')?.textContent).toBe('Un titre')
  expect(documentContent(editor.getJSON())).toEqual(state(original).document)
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  expect(undo.undoStack).toHaveLength(1)
  undo.undo()
  await new Promise((resolve) => setTimeout(resolve, 10))
  await probe({ action: 'inspect', update: bytes(original), expected: before })
  expect(root.get(0)).toBe(originalHeading)
  expect(root.get(1)).toBe(originalParagraph)
  undo.redo()
  await new Promise((resolve) => setTimeout(resolve, 10))
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  expect(editor.view.dom.querySelectorAll('li')).toHaveLength(2)
  undo.destroy()
})

it('adds, reorders and removes native header records with rich fields and a single undo step', async () => {
  const original = seed('article')
  const settings = original.getMap('settings')
  const collections = settings.get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>
  const authors = collections.get('authors')!
  const definition = catalogue.types.find((t) => t.id === 'article')!.structured!
  const authorDefinition = definition.collections.find((c) => c.parameter === 'authors')!
  const author = addRecord(store(original), authorDefinition, { id: 'camille', name: 'Camille' })
  const originalName = author.get('name')
  const localizations = collections.get('localizations')!
  const localized = [...localizations.values()][0]
  const id = localized.get('id') as string
  const abstract = localized.get('abstract') as Y.XmlFragment
  const element = document.createElement('div')
  document.body.append(element)
  const ownerElement = document.createElement('div')
  document.body.append(ownerElement)
  const historyOwner = createRichZone({
    profile: 'document',
    element: ownerElement,
    label: 'Body',
    binding: { kind: 'document', document: original },
  })
  editors.push(historyOwner)
  const editor = createRichZone({
    profile: 'rich-text',
    element,
    label: 'Abstract',
    binding: { kind: 'fragment', fragment: abstract, historyOwner },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const before = state(original)
  const origin = Symbol('native-records')
  const undo = new Y.UndoManager(settings, { trackedOrigins: new Set([origin]) })
  const guard = { document: 'records', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: [
      {
        id: 'record.add',
        params: {
          collection: 'affiliations',
          id: 'irmar',
          values: { text: 'Univ Rennes, CNRS, IRMAR' },
        },
      },
      {
        id: 'record.add',
        params: {
          collection: 'authors',
          id: 'alex',
          after: 'camille',
          values: { name: 'Alex 日本', 'affiliation-ids': ['irmar'] },
        },
      },
      { id: 'record.move', params: { collection: 'authors', id: 'alex', before: 'camille' } },
      {
        id: 'block.insert',
        params: {
          parent: {
            root: { kind: 'record', collection: 'localizations', id, field: 'abstract' },
            path: [],
          },
          index: 1,
          node: {
            type: 'paragraph',
            content: [{ type: 'text', text: 'Résumé partagé', marks: [{ type: 'italic' }] }],
          },
        },
      },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(result.update), origin)
  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(authors.get('camille')).toBe(author)
  expect(author.get('name')).toBe(originalName)
  expect(localized.get('abstract')).toBe(abstract)
  expect(editor.view.dom.querySelector('em')?.textContent).toBe('Résumé partagé')
  const alex = authors.get('alex')!
  expect(alex.get('name')).toBeInstanceOf(Y.Text)
  expect((state(original).settings.metadata.fields.authors as { id: string }[])[0].id).toBe('alex')
  expect(undo.undoStack).toHaveLength(1)
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  undo.undo()
  await new Promise((resolve) => setTimeout(resolve, 10))
  await probe({ action: 'inspect', update: bytes(original), expected: before })
  undo.redo()
  await new Promise((resolve) => setTimeout(resolve, 10))
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  const next = await probe({
    action: 'commands',
    update: bytes(original),
    guard: result.guard,
    expected: result.guard,
    commands: [
      { id: 'record.remove', params: { collection: 'authors', id: 'alex' } },
      { id: 'record.remove', params: { collection: 'affiliations', id: 'irmar' } },
    ],
  })
  undo.stopCapturing()
  Y.applyUpdate(original, new Uint8Array(next.update), origin)
  expect(authors.has('alex')).toBe(false)
  undo.undo()
  await new Promise((resolve) => setTimeout(resolve, 10))
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  undo.destroy()
})

it('roundtrips table captions and fine grid edits as one undo without replacing untouched shared cells', async () => {
  const original = seed('courrier')
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Table contract',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const guard = { document: 'table-contract', revision: 0 }
  const inserted = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: [
      {
        id: 'insert.table',
        params: { parent: { root: { kind: 'body' } }, index: 1, rows: 2, columns: 2, header: true },
      },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(inserted.update))
  const before = state(original)
  const root = original.getXmlFragment('default')
  const figure = root.get(1) as Y.XmlElement,
    grid = figure.get(0) as Y.XmlElement,
    caption = figure.get(1)
  const row = grid.get(1) as Y.XmlElement,
    cell = row.get(1)
  const origin = Symbol('table-batch')
  const undo = new Y.UndoManager(root, { trackedOrigins: new Set([origin]) })
  const target = { root: { kind: 'body' }, path: [1] }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard: inserted.guard,
    expected: inserted.guard,
    commands: [
      {
        id: 'text.insert',
        params: {
          target: { zone: { kind: 'body', path: [1, 1] }, from: 0, to: 0 },
          text: 'Résultats 日本',
          marks: [{ type: 'italic' }],
        },
      },
      {
        id: 'text.insert',
        params: {
          target: { zone: { kind: 'body', path: [1, 0, 1, 1, 0] }, from: 0, to: 0 },
          text: 'é😀',
        },
      },
      { id: 'table.row.insert', params: { target, index: 1 } },
      { id: 'table.column.insert', params: { target, index: 1 } },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(result.update), origin)
  await new Promise((resolve) => setTimeout(resolve, 10))
  expect(root.get(1)).toBe(figure)
  expect(figure.get(0)).toBe(grid)
  expect(figure.get(1)).toBe(caption)
  expect(grid.get(2)).toBe(row)
  expect(row.get(2)).toBe(cell)
  expect(editor.view.dom.querySelector('[data-table] figcaption em')?.textContent).toBe(
    'Résultats 日本',
  )
  expect(editor.view.dom.querySelector('[data-table]')!.querySelectorAll('tr')).toHaveLength(3)
  expect(undo.undoStack).toHaveLength(1)
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  undo.undo()
  await probe({ action: 'inspect', update: bytes(original), expected: before })
  undo.redo()
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
  undo.destroy()
})

it('publishes actual header form plans, reorders stable records and undoes explicit reference detachment', async () => {
  const original = seed('article'),
    storeValue = store(original)
  const definitions = catalogue.types.find((type) => type.id === 'article')!.structured!
  const authors = definitions.collections.find((definition) => definition.parameter === 'authors')!
  const affiliations = definitions.collections.find(
    (definition) => definition.parameter === 'affiliations',
  )!
  const localizations = definitions.collections.find(
    (definition) => definition.parameter === 'localizations',
  )!
  const settings = original.getMap('settings')
  let guard = { document: 'header-form-contract', revision: 0 }
  const origin = Symbol('header-command'),
    undo = new Y.UndoManager(settings, { trackedOrigins: new Set([origin]) })
  const publish = async (commands: unknown[]) => {
    const result = await probe({
      action: 'commands',
      update: bytes(original),
      guard,
      expected: guard,
      commands,
    })
    Y.applyUpdate(original, new Uint8Array(result.update), origin)
    guard = result.guard
    undo.stopCapturing()
    return result
  }
  const confirm = async (
    definition: typeof authors,
    fields: Record<string, unknown>,
    id?: string,
  ) => {
    const draft = prepareRecordEdit(storeValue, definitions, definition, { id })
    try {
      for (const [field, value] of Object.entries(fields))
        setRecordField(draft, draft.record, field, value)
      const plan = draft.plan()
      expect(plan.error).toBeUndefined()
      await publish(plan.commands!)
      return plan.id!
    } finally {
      draft.destroy()
    }
  }
  try {
    const aff = await confirm(affiliations, { text: 'Univ Rennes, CNRS, IRMAR' })
    const first = await confirm(authors, { name: 'Camille 😀', 'affiliation-ids': [aff] })
    const second = await confirm(authors, { name: 'Alex 日本' })
    const firstMap = recordMap(storeValue, authors).get(first)!
    const name = firstMap.get('name')
    await confirm(authors, { name: 'Dr Camille 😀', email: 'camille@example.org' }, first)
    expect(recordMap(storeValue, authors).get(first)).toBe(firstMap)
    expect(firstMap.get('name')).toBe(name)
    await publish(moveRecordCommands(storeValue, authors, second, -1))
    expect(
      (settingsFor(storeValue).metadata.fields.authors as { id: string }[]).map(
        (author) => author.id,
      ),
    ).toEqual([second, first])
    const localization = [...recordMap(storeValue, localizations).values()][0]
    const fragment = localization.get('abstract')
    await confirm(
      localizations,
      {
        abstract: {
          type: 'doc',
          content: [
            {
              type: 'paragraph',
              content: [{ type: 'text', text: 'Résumé relu', marks: [{ type: 'italic' }] }],
            },
          ],
        },
      },
      String(localization.get('id')),
    )
    expect(localization.get('abstract')).toBe(fragment)
    expect(removeRecordCommands(storeValue, definitions, affiliations, aff)).toBeNull()
    const before = state(original)
    const removal = removeRecordCommands(storeValue, definitions, affiliations, aff, true)!
    expect(removal.map((command) => command.id)).toEqual(['metadata.set', 'record.remove'])
    await publish(removal)
    expect(settingsFor(storeValue).metadata.fields.affiliations as unknown[]).toHaveLength(0)
    undo.undo()
    expect(state(original)).toEqual(before)
    await probe({ action: 'inspect', update: bytes(original), expected: before })
  } finally {
    undo.destroy()
  }
})

it('executes GUI formatting targets in Rust across body badges and record rich fields', async () => {
  const original = seed('article')
  const localization = [
    ...(original.getMap('settings').get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>)
      .get('localizations')!
      .values(),
  ][0]
  setRecordField(store(original), localization, 'abstract', {
    type: 'doc',
    content: [
      { type: 'paragraph', content: [{ type: 'text', text: 'Résumé 😀' }] },
      { type: 'paragraph', content: [{ type: 'text', text: 'Autre paragraphe' }] },
    ],
  })
  const editor = createRichZone({
    profile: 'document',
    label: 'Body targets',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const root = original.getXmlFragment('default'),
    paragraph = root.get(1)
  const fragment = localization.get('abstract') as Y.XmlFragment
  const field = createRichZone({
    profile: 'rich-text',
    label: 'Record targets',
    binding: {
      kind: 'fragment',
      fragment,
      historyOwner: editor,
      target: {
        kind: 'record',
        collection: 'localizations',
        id: String(localization.get('id')),
        field: 'abstract',
      },
    },
  })
  editors.push(field)
  if (!field.isInitialized)
    await new Promise<void>((resolve) => field.once('create', () => resolve()))
  editor.commands.selectAll()
  field.commands.selectAll()
  const commands = [
    ...formattingBatch(editor, 'text.italic', editor.state.selection),
    ...formattingBatch(field, 'text.bold', field.state.selection),
  ]
  const guard = { document: 'gui-formatting', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands,
  })
  Y.applyUpdate(original, new Uint8Array(result.update), 'native-formatting')
  expect(root.get(1)).toBe(paragraph)
  expect(localization.get('abstract')).toBe(fragment)
  expect(field.view.dom.querySelector('strong')?.textContent).toBe('Résumé 😀')
  const marks = documentContent(editor.getJSON()).content![1].content!
  expect(
    marks
      .filter((n) => ['footnote', 'citation'].includes(n.type!))
      .every((n) => n.marks?.some((m) => m.type === 'italic')),
  ).toBe(true)
  await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
})

it('executes literal replacements and a document spelling exception across body and abstract with one undo', async () => {
  const original = seed('article')
  const localization = [
    ...(original.getMap('settings').get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>)
      .get('localizations')!
      .values(),
  ][0]
  setRecordField(store(original), localization, 'abstract', {
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: [{ type: 'text', text: 'Citation 日本 Citation', marks: [{ type: 'bold' }] }],
      },
    ],
  })
  const editor = createRichZone({
    profile: 'document',
    label: 'Proofing body',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const fragment = localization.get('abstract') as Y.XmlFragment
  const field = createRichZone({
    profile: 'rich-text',
    label: 'Proofing abstract',
    binding: {
      kind: 'fragment',
      fragment,
      historyOwner: editor,
      target: {
        kind: 'record',
        collection: 'localizations',
        id: String(localization.get('id')),
        field: 'abstract',
      },
    },
  })
  editors.push(field)
  if (!field.isInitialized)
    await new Promise<void>((resolve) => field.once('create', () => resolve()))
  const root = original.getXmlFragment('default'),
    quote = root.get(2),
    before = state(original)
  const commands = [editor, field].flatMap(
    (zone) =>
      replacementPlan(
        zone,
        findMatches(zone.state.doc, 'Citation', true).map((hit) => ({ ...hit, text: 'Analyse' })),
      ).commands,
  )
  commands.push({
    id: 'spelling.ignore',
    params: {
      target: { zone: { kind: 'body', path: [2, 0] }, from: 0, to: 7 },
      language: 'fr',
      word: 'Analyse',
    },
  })
  const guard = { document: 'proofing-contract', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands,
  })
  const undo = new Y.UndoManager([root, original.getMap('settings')], {
    trackedOrigins: new Set(['proofing-command']),
  })
  try {
    Y.applyUpdate(original, new Uint8Array(result.update), 'proofing-command')
    expect(root.get(2)).toBe(quote)
    expect(localization.get('abstract')).toBe(fragment)
    expect(field.view.dom.querySelector('strong')?.textContent).toBe('Analyse 日本 Analyse')
    expect(settingsFor(store(original)).metadata.fields.options.spelling_ignored).toEqual({
      fr: ['analyse'],
    })
    expect(
      editor.state.doc.child(1).content.content.filter((node) => node.isAtom && !node.isText),
    ).toHaveLength(2)
    expect(undo.undoStack).toHaveLength(1)
    await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
    const after = state(original)
    undo.undo()
    expect(state(original)).toEqual(before)
    undo.redo()
    expect(state(original)).toEqual(after)
  } finally {
    undo.destroy()
  }
})

it('executes widget splices and inline object parameters generated by the web adapter without replacing neighboring identities', async () => {
  const original = seed('article')
  const editor = createRichZone({
    profile: 'document',
    label: 'Object targets',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const root = original.getXmlFragment('default'),
    heading = root.get(0)
  const quote = editor.schema.nodeFromJSON({
    type: 'quote',
    content: [
      { type: 'paragraph', content: [{ type: 'text', text: 'Citation' }] },
      { type: 'quoteAttribution' },
    ],
  })
  const guard = { document: 'gui-objects', revision: 0 }
  const first = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: widgetBatch(editor, editor.state.tr.insert(0, quote)),
  })
  Y.applyUpdate(original, new Uint8Array(first.update), 'native-objects')
  expect(root.get(1)).toBe(heading)
  const from = editor.state.doc.child(0).nodeSize + 1 + 3
  const target = inlineTarget(editor, from, from)
  const second = await probe({
    action: 'commands',
    update: bytes(original),
    guard: first.guard,
    expected: first.guard,
    commands: [
      {
        id: 'inline.replace',
        params: {
          target,
          node: { type: 'footnote', attrs: { text: 'Note 😀' }, marks: [{ type: 'italic' }] },
        },
      },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(second.update), 'native-objects')
  const paragraph = root.get(1) as Y.XmlElement
  const note = paragraph
    .toArray()
    .find((node) => (node as Y.XmlElement).nodeName === 'footnote') as Y.XmlElement
  const third = await probe({
    action: 'commands',
    update: bytes(original),
    guard: second.guard,
    expected: second.guard,
    commands: [
      {
        id: 'node.set_attrs',
        params: { target: treeTarget(editor, from), attrs: { text: 'Note relue' } },
      },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(third.update), 'native-objects')
  expect(root.get(1)).toBe(paragraph)
  expect(paragraph.toArray()).toContain(note)
  expect(note.getAttribute('text')).toBe('Note relue')
  expect(
    editor.view.dom.querySelector('.footnote-ref .inline-object-label')?.getAttribute('title'),
  ).toContain('Note relue')
  await probe({ action: 'inspect', update: bytes(original), expected: state(original) })
})

it('uses isolated native participant contexts and selective undo in the actual Yjs/Tiptap document', async () => {
  const original = seed('article')
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Participant history contract',
    binding: { kind: 'document', document: original },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  editor.commands.setTextSelection(2) // The GUI cursor is independent of both native contexts.
  const before = state(original)
  const localization = (before.settings.metadata.fields.localizations as { id: string }[])[0].id
  const body = (from: number, to = from) => ({ zone: { kind: 'body', path: [1] }, from, to })
  const abstract = {
    zone: {
      kind: 'record',
      collection: 'localizations',
      id: localization,
      field: 'abstract',
      path: [0],
    },
    from: 0,
    to: 0,
  }
  const result = await probe({
    action: 'editing',
    update: bytes(original),
    guard: { document: 'participant-contract', revision: 0 },
    contexts: 2,
    steps: [
      { op: 'select', actor: 0, target: body(3) },
      { op: 'style', actor: 0, style: { type: 'bold' }, value: 'on' },
      { op: 'insert', actor: 0, text: ' Humain ' },
      { op: 'select', actor: 1, target: body(0) },
      { op: 'insert', actor: 1, text: 'Agent ' },
      { op: 'history', actor: 0, direction: 'undo' },
      { op: 'history', actor: 0, direction: 'redo' },
      { op: 'select', actor: 0, target: abstract },
      { op: 'style', actor: 0, style: { type: 'italic' }, value: 'on' },
      { op: 'insert', actor: 0, text: 'Résumé 日本' },
      { op: 'history', actor: 0, direction: 'undo' },
      { op: 'history', actor: 0, direction: 'redo' },
    ],
  })
  expect(result.events[1].contexts[0].typing_styles).toEqual([{ type: 'bold' }])
  expect(result.events[1].contexts[1].typing_styles).toEqual([])
  expect(result.events[2].contexts[0].undo_steps).toBe(1)
  for (const event of result.events) {
    if (event.publication?.changed) {
      Y.applyUpdate(original, new Uint8Array(event.publication.update), 'native-confirmation')
      await probe({ action: 'inspect', update: bytes(original), expected: event.publication.state })
      expect(editor.state.selection.from).toBe(2)
    }
  }
  expect(editor.view.dom.querySelector('strong')?.textContent).toBeTruthy()
  expect(editor.view.dom.textContent).toContain('Agent ')
  expect(editor.view.dom.textContent).toContain(' Humain ')
  const after = state(original)
  const localizations = after.settings.metadata.fields.localizations as { abstract: JSONContent }[]
  expect(JSON.stringify(localizations[0].abstract)).toContain('Résumé 日本')
  expect(result.events[11].contexts[0].undo_steps).toBe(2)
  expect(result.events[11].contexts[1].undo_steps).toBe(1)
  expect(
    result.events[5].publication.state.document.content[1].content
      .map((n: JSONContent) => n.text || '')
      .join(''),
  ).not.toContain(' Humain ')
  expect(
    result.events[5].publication.state.document.content[1].content
      .map((n: JSONContent) => n.text || '')
      .join(''),
  ).toContain('Agent ')
  await probe({ action: 'inspect', update: bytes(original), expected: result.state })
})

it('model and language commands preserve Article branches and undo the whole language transition', async () => {
  const original = seed('article')
  const definition = catalogue.types
    .find((t) => t.id === 'article')!
    .structured!.collections.find((c) => c.parameter === 'authors')!
  addRecord(store(original), definition, { name: 'Camille Martin' })
  const before = state(original)
  const collections = original.getMap('settings').get('collections') as Y.Map<Y.Map<Y.Map<unknown>>>
  const authors = collections.get('authors')!,
    author = authors.values().next().value!
  const name = author.get('name')
  const locales = collections.get('localizations')!,
    primary = locales.values().next().value!
  const title = primary.get('title')
  const guard = { document: 'models-options', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(original),
    guard,
    expected: guard,
    commands: [
      { id: 'metadata.set', params: { target: { kind: 'option', name: 'paper' }, value: 'a5' } },
      { id: 'metadata.set', params: { target: { kind: 'option', name: 'language' }, value: 'en' } },
      {
        id: 'document.set_model',
        params: {
          model_id: 'article.project',
          source: before.settings.template + '\n// retained project model',
        },
      },
    ],
  })
  Y.applyUpdate(original, new Uint8Array(result.update))
  expect(original.getMap('settings').get('collections')).toBe(collections)
  expect(author.get('name')).toBe(name)
  expect(primary.get('title')).toBe(title)
  const final = state(original)
  expect(final.document).toEqual(before.document)
  expect(final.settings.metadata.fields.options.language).toBe('en')
  expect(final.settings.metadata.fields.options.paper).toBe('a5')
  expect(final.settings.metadata.fields.localizations).toHaveLength(2)
  await probe({ action: 'inspect', update: bytes(original), expected: final })
  const history = await probe({
    action: 'editing',
    update: bytes(original),
    guard: result.guard,
    contexts: 1,
    steps: [
      {
        op: 'commands',
        actor: 0,
        commands: [
          {
            id: 'metadata.set',
            params: { target: { kind: 'option', name: 'language' }, value: 'de' },
          },
        ],
      },
      { op: 'history', actor: 0, direction: 'undo' },
      { op: 'history', actor: 0, direction: 'redo' },
    ],
  })
  expect(history.events[0].contexts[0].undo_steps).toBe(1)
  expect(history.events[1].publication.state.settings.metadata.fields.localizations).toHaveLength(2)
  expect(history.state.settings.metadata.fields.localizations).toHaveLength(3)
})

it('immutable images and signatures share one guarded batch and remain available for redo', async () => {
  const original = seed('courrier')
  const resource = await call('prepare_image', {
    extension: 'svg',
    data: Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" width="11" height="12"><rect width="11" height="12" fill="green"/></svg>',
    ).toString('base64'),
  })
  const guard = { document: 'resource-contract', revision: 0 }
  const result = await probe({
    action: 'editing',
    update: bytes(original),
    guard,
    contexts: 2,
    steps: [
      {
        op: 'commands',
        actor: 0,
        commands: [
          { id: 'resource.add', params: { resource } },
          { id: 'document.set_signature', params: { path: resource.path } },
        ],
      },
      { op: 'history', actor: 0, direction: 'undo' },
      { op: 'history', actor: 0, direction: 'redo' },
    ],
  })
  expect(result.events[0].publication.resources).toEqual([resource])
  expect(result.events[0].contexts[0].undo_steps).toBe(1)
  expect(result.events[0].contexts[1].undo_steps).toBe(0)
  expect(result.events[1].publication.state.settings.metadata.fields.signature).toBeNull()
  expect(result.state.settings.metadata.fields.signature).toBe(resource.path)
  const body = original.getXmlFragment('default'),
    nodes = body.toArray()
  Y.applyUpdate(original, new Uint8Array(result.update))
  expect(body.toArray()).toEqual(nodes)
  expect(body.toArray().every((node, index) => node === nodes[index])).toBe(true)
  expect(state(original).settings.metadata.fields.signature).toBe(resource.path)
})

it('attached MCP accepts the same typed lot into the existing Yjs tree and human history', async () => {
  const original = seed('article')
  await call('reset', {})
  const joined = await call('join', {})
  const generation = joined.generation
  await call('relay', { generation, update: bytes(original), capture: false })
  const body = original.getXmlFragment('default')
  const nodes = body.toArray()
  const agent = (method: string, params: unknown) => call('diagnostic_agent', { method, params })
  const inspect = await agent('document.inspect', {})
  const before = state(original)
  const selection = {
    kind: 'text',
    anchor: { zone: { kind: 'body', path: [1] }, offset: 1 },
    head: { zone: { kind: 'body', path: [1] }, offset: 1 },
  }
  // Agent navigation stays private; it is never translated into human focus.
  await agent('context.set', { document: inspect.document, selection, typing_styles: [] })
  const pending = await agent('proposals.submit', {
    document: inspect.document,
    request_id: 'contract-one',
    summary: 'Un ajout localisé',
    steps: [
      {
        id: 'text.insert',
        params: {
          target: { zone: { kind: 'body', path: [1] }, from: 1, to: 1 },
          text: ' Ajout ',
          marks: [{ type: 'italic' }],
        },
      },
      {
        id: 'insert.table',
        params: { parent: { root: { kind: 'body' } }, index: 2, rows: 1, columns: 2, header: true },
      },
    ],
  })
  expect((await agent('document.inspect', {})).outcome.state).toEqual(inspect.outcome.state)
  const publication = await call('diagnostic_agent_accept', { id: pending.outcome.id })
  expect(publication.guard.revision).toBe(inspect.document.guard.revision + 1)
  Y.applyUpdate(original, new Uint8Array(publication.update))
  expect(body.toArray()[0]).toBe(nodes[0])
  expect(body.toArray()[1]).toBe(nodes[1])
  await probe({ action: 'inspect', update: bytes(original), expected: publication.state })
  expect(state(original).settings).toEqual(before.settings)
  expect((await call('editing_context', { generation })).undo_steps).toBe(1)
  const undo = await call('editing_history', { generation, direction: 'undo' })
  Y.applyUpdate(original, new Uint8Array(undo.update))
  expect(state(original)).toEqual(before)
  const redo = await call('editing_history', { generation, direction: 'redo' })
  Y.applyUpdate(original, new Uint8Array(redo.update))
  await probe({ action: 'inspect', update: bytes(original), expected: publication.state })
})

it('the immediate prose join and typed native join agree on Unicode, badges, marks and block kinds', async () => {
  const { joinParagraphs } = await import('../src/paragraph-join')
  for (const [leftKind, rightKind] of [
    ['paragraph', 'paragraph'],
    ['heading', 'paragraph'],
    ['paragraph', 'heading'],
    ['heading', 'heading'],
  ]) {
    const initial = seed('article')
    const root = initial.getXmlFragment('default')
    const paragraph = (type: string, content: JSONContent[]) => ({
      type,
      ...(type === 'heading' ? { attrs: { level: 2 } } : {}),
      content,
    })
    const content = {
      type: 'doc',
      content: [
        paragraph(leftKind, [{ type: 'text', text: 'A😀', marks: [{ type: 'bold' }] }]),
        paragraph(rightKind, [
          { type: 'footnote', attrs: { text: 'Note 日本' } },
          {
            type: 'text',
            text: 'é日本',
            marks: [{ type: 'italic' }, { type: 'link', attrs: { href: 'https://example.org' } }],
          },
        ]),
        { type: 'paragraph' },
      ],
    }
    initial.transact(() => {
      root.delete(0, root.length)
      prosemirrorJSONToYXmlFragment(schema, content, root)
    })
    const native = doc(bytes(initial)),
      local = doc(bytes(initial))
    const editor = createRichZone({
      profile: 'document',
      label: 'Join contract',
      binding: { kind: 'document', document: local },
    })
    editors.push(editor)
    if (!editor.isInitialized)
      await new Promise<void>((resolve) => editor.once('create', () => resolve()))
    editor.commands.setTextSelection(6)
    expect(joinParagraphs(editor, true, true)).toBe(true)
    expect(editor.state.selection.from).toBe(
      leftKind === 'paragraph' && rightKind === 'paragraph' ? 5 : 4,
    )
    const guard = { document: 'join-contract', revision: 0 }
    const outcome = await probe({
      action: 'commands',
      update: bytes(native),
      guard,
      expected: guard,
      commands: [
        {
          id: 'block.join',
          params: {
            left: { root: { kind: 'body' }, path: [0] },
            right: { root: { kind: 'body' }, path: [1] },
          },
        },
      ],
    })
    Y.applyUpdate(native, new Uint8Array(outcome.update))
    expect(state(native)).toEqual(state(local))
    await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
  }
})

it('native and local whitespace repair preserve Unicode, marks and NBSP in body and rich header fields', async () => {
  const { normalizeSharedSpaces } = await import('../src/horizontal-space')
  const initial = seed('article'),
    native = doc(bytes(initial)),
    local = doc(bytes(initial))
  const rich = {
    type: 'doc',
    content: [
      {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'A😀  ', marks: [{ type: 'bold' }] },
          { type: 'text', text: '  Bé\u00a0\u00a0fin', marks: [{ type: 'italic' }] },
        ],
      },
    ],
  }
  const root = local.getXmlFragment('default')
  const definition = catalogue.types
    .find((t) => t.id === 'article')!
    .structured!.collections.find((c) => c.parameter === 'localizations')!
  const localizations = recordMap(store(local), definition)
  const localized = Array.from(localizations.values())[0] as Y.Map<unknown>
  const id = Array.from(localizations.keys())[0]
  for (const target of [root, localized.get('title'), localized.get('abstract')]) {
    const fragment = target as Y.XmlFragment
    fragment.doc!.transact(() => {
      fragment.delete(0, fragment.length)
      prosemirrorJSONToYXmlFragment(schema, rich, fragment)
      normalizeSharedSpaces(fragment)
    })
  }
  // The body retains its mandatory editing slot; rich headers do not gain one.
  local.transact(() => root.insert(root.length, [new Y.XmlElement('paragraph')]))
  const guard = { document: 'space-contract', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(native),
    guard,
    expected: guard,
    commands: [
      {
        id: 'block.replace',
        params: { target: { root: { kind: 'body' }, path: [0] }, node: rich.content[0] },
      },
      ...['title', 'abstract'].map((field) => ({
        id: 'block.replace',
        params: {
          target: { root: { kind: 'record', collection: 'localizations', id, field }, path: [0] },
          node: rich.content[0],
        },
      })),
      {
        id: 'block.splice',
        params: {
          parent: { root: { kind: 'body' }, path: [] },
          index: 1,
          delete_count: state(native).document.content!.length - 1,
          nodes: [{ type: 'paragraph' }],
        },
      },
    ],
  })
  Y.applyUpdate(native, new Uint8Array(result.update))
  expect(state(native)).toEqual(state(local))
  await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
})

it('header axes round-trip between native commands, Yjs and the editor without losing cell text', async () => {
  const native = seed('courrier')
  const guard = { document: 'table-header-contract', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(native),
    guard,
    expected: guard,
    commands: [
      {
        id: 'insert.table',
        params: {
          parent: { root: { kind: 'body' }, path: [] },
          index: 0,
          rows: 2,
          columns: 2,
          header: false,
          row_headers: true,
        },
      },
      {
        id: 'text.insert',
        params: {
          target: { zone: { kind: 'body', path: [0, 0, 1, 0, 0] }, from: 0, to: 0 },
          text: 'Nom é😀日本',
          marks: [{ type: 'bold' }],
        },
      },
      {
        id: 'table.header',
        params: { target: { root: { kind: 'body' }, path: [0] }, axis: 'columns', enabled: true },
      },
    ],
  })
  Y.applyUpdate(native, new Uint8Array(result.update))
  const projected = state(native)
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile: 'document',
    element,
    label: 'Table header contract',
    content: projected.document,
  })
  editors.push(editor)
  const table = (editor.getJSON() as JSONContent).content![0].content![0]
  expect(table.content![0].content![0].attrs!.scope).toBe('both')
  expect(table.content![1].content![0].attrs!.scope).toBe('row')
  expect(table.content![1].content![0].content![0].content![0]).toMatchObject({
    text: 'Nom é😀日本',
    marks: [{ type: 'bold' }],
  })
  const roundtrip = doc(bytes(native))
  const fragment = roundtrip.getXmlFragment('default')
  roundtrip.transact(() => {
    fragment.delete(0, fragment.length)
    prosemirrorJSONToYXmlFragment(schema, editor.getJSON(), fragment)
  })
  await probe({ action: 'inspect', update: bytes(roundtrip), expected: projected })
})

it.each(['softBreak', 'hardBreak'])(
  'inline.replace inserts %s exactly like the immediate rich editor, preserving neighboring identities and undo',
  async (kind) => {
    for (const path of [[1], [2, 0], [2, 1], [3, 0, 0], [4, 0, 0, 0, 0], [5]]) {
      const original = seed('courrier')
      const native = doc(bytes(original)),
        local = doc(bytes(original))
      const editor = createRichZone({
        profile: 'document',
        label: 'Break contract',
        binding: { kind: 'document', document: local },
      })
      editors.push(editor)
      if (!editor.isInitialized)
        await new Promise<void>((resolve) => editor.once('create', () => resolve()))
      // Pick only actual inline zones from the fixture; tables are wrapped by the schema.
      const start = positionAtPath(editor, path)
      const node = editor.state.doc.nodeAt(start)!
      expect(node.isTextblock).toBe(true)
      const offset = 1
      editor.commands.setTextSelection(start + 1 + offset)
      editor.commands.insertContent({ type: kind })
      const guard = { document: 'break-contract', revision: 0 }
      const result = await probe({
        action: 'commands',
        update: bytes(native),
        guard,
        expected: guard,
        commands: [
          {
            id: 'inline.replace',
            params: {
              target: { zone: { kind: 'body', path }, from: offset, to: offset },
              node: { type: kind },
            },
          },
        ],
      })
      const undo = new Y.UndoManager(native.getXmlFragment('default'))
      Y.applyUpdate(native, new Uint8Array(result.update))
      expect(state(native)).toEqual(state(local))
      await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
      const confirmed = state(native)
      undo.undo()
      expect(state(native)).toEqual(state(original))
      undo.redo()
      expect(state(native)).toEqual(confirmed)
      undo.destroy()
    }
  },
)

it('flat item plans and native operations agree on split, symmetric join, exit, rich content and one undo', async () => {
  const { planListSplit, planListJoin, planListExit } = await import('../src/list-transforms')
  for (const kind of ['bulletList', 'orderedList', 'descriptionList']) {
    for (const action of ['split', 'join', 'exit']) {
      if (kind === 'descriptionList' && action === 'join') continue
      const initial = seed('courrier')
      const root = initial.getXmlFragment('default')
      const paragraph = (text: string) => ({
        type: 'paragraph',
        content: text ? [{ type: 'text', text, marks: [{ type: 'italic' }] }] : [],
      })
      const node = {
        type: kind,
        content: ['A😀BC', action === 'exit' ? '' : 'Second', 'Voisin'].map((text) =>
          kind === 'descriptionList'
            ? {
                type: 'descriptionItem',
                content: [
                  { type: 'descriptionTerm' },
                  {
                    type: 'descriptionDefinition',
                    content: text ? [{ type: 'text', text, marks: [{ type: 'bold' }] }] : [],
                  },
                ],
              }
            : {
                type: 'listItem',
                content: [
                  paragraph(text),
                  ...(action === 'split' && text === 'A😀BC'
                    ? [paragraph('Deuxième paragraphe')]
                    : []),
                ],
              },
        ),
      }
      initial.transact(() => {
        root.delete(0, root.length)
        prosemirrorJSONToYXmlFragment(
          schema,
          { type: 'doc', content: [node, { type: 'paragraph' }] },
          root,
        )
      })
      const native = doc(bytes(initial)),
        local = doc(bytes(initial))
      const editor = createRichZone({
        profile: 'document',
        label: 'Item contract',
        binding: { kind: 'document', document: local },
      })
      editors.push(editor)
      if (!editor.isInitialized)
        await new Promise<void>((resolve) => editor.once('create', () => resolve()))
      const path = [0, action === 'exit' ? 1 : 0, kind === 'descriptionList' ? 1 : 0]
      const start = positionAtPath(editor, path)
      editor.commands.setTextSelection(
        start + 1 + (action === 'split' ? 3 : action === 'join' ? 5 : 0),
      )
      const plan =
        action === 'split'
          ? planListSplit(editor)
          : action === 'join'
            ? planListJoin(editor, false)
            : planListExit(editor)
      expect(plan).toBeDefined()
      editor.view.dispatch(plan!.transaction)
      const guard = { document: 'list-contract', revision: 0 }
      const result = await probe({
        action: 'commands',
        update: bytes(native),
        guard,
        expected: guard,
        commands: plan!.commands,
      })
      const undo = new Y.UndoManager(native.getXmlFragment('default'))
      Y.applyUpdate(native, new Uint8Array(result.update))
      expect(state(native), kind + ' ' + action).toEqual(state(local))
      await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
      const confirmed = state(native)
      expect(undo.undoStack).toHaveLength(1)
      undo.undo()
      expect(state(native)).toEqual(state(initial))
      undo.redo()
      expect(state(native)).toEqual(confirmed)
      undo.destroy()
    }
  }
})

it('protected multizone replacement plans match one native atomic lot without deleting partial widget boundaries', async () => {
  const { protectedSelectionPlan } = await import('../src/selection-edits')
  const { DOMParser, Slice } = await import('@tiptap/pm/model')
  for (const [first, last] of [
    ['Corps', 'Source'],
    ['Terme', 'Définition'],
    ['Alpha', 'Bravo'],
    ['Avant', 'Définition'],
  ]) {
    for (const paste of [false, true]) {
      const initial = seed('courrier'),
        root = initial.getXmlFragment('default')
      const element = document.createElement('div')
      element.innerHTML =
        '<p>Avant</p><blockquote><p>Corps</p><cite data-attribution>Source</cite></blockquote><dl><div data-description-item><dt>Terme</dt><dd>Définition</dd></div></dl><figure data-table><table><tbody><tr><td><p>Alpha</p></td><td><p>Bravo</p></td></tr></tbody></table><figcaption data-table-caption>Légende</figcaption></figure><p>Après</p><p></p>'
      const content = DOMParser.fromSchema(schema).parse(element)
      initial.transact(() => {
        root.delete(0, root.length)
        prosemirrorJSONToYXmlFragment(schema, content.toJSON(), root)
      })
      const native = doc(bytes(initial)),
        local = doc(bytes(initial))
      const editor = createRichZone({
        profile: 'document',
        label: 'Protected selection contract',
        binding: { kind: 'document', document: local },
      })
      editors.push(editor)
      if (!editor.isInitialized)
        await new Promise<void>((resolve) => editor.once('create', () => resolve()))
      let from = 0,
        to = 0
      editor.state.doc.descendants((node, pos) => {
        if (node.isTextblock) {
          if (node.textContent === first) from = pos + 2
          if (node.textContent === last) to = pos + 3
        }
      })
      editor.commands.setTextSelection({ from, to })
      const payload = document.createElement('div')
      payload.innerHTML = '<p><strong>Ajout</strong></p><p><em>Suite</em></p>'
      const plan = protectedSelectionPlan(
        editor,
        paste ? DOMParser.fromSchema(editor.schema).parseSlice(payload) : Slice.empty,
      )!
      expect(plan).toBeDefined()
      editor.view.dispatch(plan.transaction)
      const guard = { document: 'selection-contract', revision: 0 }
      const result = await probe({
        action: 'commands',
        update: bytes(native),
        guard,
        expected: guard,
        commands: plan.commands,
      })
      const undo = new Y.UndoManager(native.getXmlFragment('default'))
      Y.applyUpdate(native, new Uint8Array(result.update))
      expect(state(native), first + ' / ' + last).toEqual(state(local))
      await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
      expect(undo.undoStack).toHaveLength(1)
      undo.undo()
      expect(state(native)).toEqual(state(initial))
      undo.destroy()
    }
  }
})

it('rectangular paste uses native cell-content splices without changing grid identities or header roles', async () => {
  const { cellPastePlan } = await import('../src/cell-paste')
  const { DOMParser } = await import('@tiptap/pm/model')
  const initial = seed('courrier'),
    root = initial.getXmlFragment('default')
  const html = document.createElement('div')
  html.innerHTML =
    '<figure data-table><table><tr><th data-header-scope="both"><p>Nom</p></th><th data-header-scope="column"><p>Valeur</p></th></tr><tr><th data-header-scope="row"><p>Ligne</p></th><td><p>Ancien</p></td></tr></table><figcaption data-table-caption>Légende</figcaption></figure><p></p>'
  initial.transact(() => {
    root.delete(0, root.length)
    prosemirrorJSONToYXmlFragment(schema, DOMParser.fromSchema(schema).parse(html).toJSON(), root)
  })
  const native = doc(bytes(initial)),
    local = doc(bytes(initial))
  const editor = createRichZone({
    profile: 'document',
    label: 'Cell paste contract',
    binding: { kind: 'document', document: local },
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const positions: number[] = []
  editor.state.doc.descendants((n, pos) => {
    if (['tableCell', 'tableHeader'].includes(n.type.name)) positions.push(pos)
  })
  editor.commands.setCellSelection({ anchorCell: positions[0], headCell: positions[3] })
  const pasted = document.createElement('div')
  pasted.innerHTML =
    '<table><tr><td><p>A</p></td><td><p>B</p></td></tr><tr><td><p>C</p></td><td><p>D</p></td></tr></table>'
  const plan = cellPastePlan(editor, DOMParser.fromSchema(editor.schema).parseSlice(pasted))!
  editor.view.dispatch(plan.transaction)
  const guard = { document: 'cell-paste-contract', revision: 0 }
  const result = await probe({
    action: 'commands',
    update: bytes(native),
    guard,
    expected: guard,
    commands: plan.commands,
  })
  Y.applyUpdate(native, new Uint8Array(result.update))
  expect(state(native)).toEqual(state(local))
  await probe({ action: 'inspect', update: bytes(local), expected: state(native) })
})

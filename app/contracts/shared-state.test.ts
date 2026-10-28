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
import { addRecord, setRecordField } from '../src/metadata-records'
import type { Provider } from '../src/provider'
import type { Catalogue } from '../src/shell'

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
  const binary = resolve(
    '../target/debug/typstographe' + (process.platform === 'win32' ? '.exe' : ''),
  )
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

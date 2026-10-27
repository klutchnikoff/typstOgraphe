import { readdirSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { afterEach, expect, it, vi } from 'vitest'
import { getSchema, type Editor } from '@tiptap/core'
import { prosemirrorJSONToYXmlFragment, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import {
  createRichZone,
  richZoneExtensions,
  richZoneOwnsShortcut,
  type RichZoneProfile,
} from '../src/rich-zone'
import { documentWhitespaceKey } from '../src/document-whitespace'

const editors: Editor[] = [],
  docs: Y.Doc[] = []
afterEach(() => {
  editors
    .splice(0)
    .reverse()
    .forEach((editor) => editor.destroy())
  docs.splice(0).forEach((doc) => doc.destroy())
  document.body.replaceChildren()
})
async function create(
  profile: RichZoneProfile,
  rest: Partial<Parameters<typeof createRichZone>[0]> = {},
) {
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({
    profile,
    element,
    label: 'Zone de test',
    language: 'fr',
    ...rest,
  })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  return editor
}
const paragraphs = (editor: Editor) => editor.state.doc.content.content.map((p) => p.textContent)
const rich = (text: string) => ({
  type: 'doc',
  content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
})

it.each(['document', 'rich-text'] as const)(
  '%s hérite des retours, de leur normalisation et des marques',
  async (profile) => {
    const editor = await create(profile, { content: '<p>Avant</p>' })
    editor.commands.setTextSelection(6)
    editor.commands.keyboardShortcut('Enter')
    expect(paragraphs(editor)).toEqual(['Avant'])
    expect(editor.state.doc.firstChild!.lastChild!.type.name).toBe('softBreak')
    editor.commands.keyboardShortcut('Enter')
    expect(paragraphs(editor)).toEqual(['Avant', ''])
    for (let i = 0; i < 4; i++) editor.commands.keyboardShortcut('Enter')
    expect(paragraphs(editor)).toEqual(['Avant', ''])
    editor.commands.setContent(
      '<p></p><p><strong>Avant</strong><br data-soft-break><br data-soft-break>Après</p><p></p><p></p><p>Suite</p>',
    )
    expect(paragraphs(editor)).toEqual(
      profile === 'document' ? ['AvantAprès', '', 'Suite', ''] : ['AvantAprès', '', 'Suite'],
    )
    expect(editor.state.doc.firstChild!.firstChild!.marks[0].type.name).toBe('bold')
    expect(editor.view.dom.getAttribute('spellcheck')).toBe('false')
    expect(editor.view.dom.getAttribute('lang')).toBe('fr')
  },
)
it('le profil en ligne bloque les retours et conserve les styles, les liens et une annulation locale', async () => {
  const openLink = vi.fn()
  const editor = await create('inline-text', { content: '<p>Titre</p>', openLink })
  editor.commands.setTextSelection(6)
  const before = editor.getJSON()
  editor.commands.keyboardShortcut('Enter')
  editor.commands.keyboardShortcut('Shift-Enter')
  expect(editor.getJSON()).toEqual(before)
  editor.chain().selectAll().toggleBold().toggleItalic().run()
  editor.commands.keyboardShortcut('Mod-k')
  expect(openLink).toHaveBeenCalledWith(editor)
  expect(editor.state.doc.firstChild!.firstChild!.marks.map((m) => m.type.name)).toEqual([
    'bold',
    'italic',
  ])
  editor.commands.undo()
  expect(editor.state.doc.textContent).toBe('Titre')
  expect(editor.state.doc.firstChild!.firstChild!.marks).toEqual([])
  expect(editor.view.dom.getAttribute('aria-multiline')).toBe('false')
  expect(richZoneOwnsShortcut(editor.view.dom, 'text.bold')).toBe(true)
  expect(richZoneOwnsShortcut(editor.view.dom, 'document.save')).toBe(false)
})
it.each(['rich-text', 'inline-text'] as const)(
  '%s refuse les widgets et utilise le même schéma au montage et à la sérialisation',
  async (profile) => {
    const editor = await create(profile, { content: '<p>Un <em>texte</em></p>' })
    const schema = getSchema(richZoneExtensions(profile))
    expect(schema.nodeFromJSON(editor.getJSON()).toJSON()).toEqual(editor.getJSON())
    for (const kind of ['heading', 'table', 'figure', 'footnote', 'quote', 'bulletList'])
      expect(() =>
        schema.nodeFromJSON({ type: 'doc', content: [{ type: kind }] }).check(),
      ).toThrow()
  },
)
it.each(['document', 'rich-text', 'inline-text'] as const)(
  '%s préserve la composition IME et ne déclenche pas de lien pendant la préédition',
  async (profile) => {
    const openLink = vi.fn()
    const editor = await create(profile, { content: '<p>にほん</p>', openLink })
    editor.commands.setTextSelection({ from: 1, to: 4 })
    Object.defineProperty(editor.view, 'composing', { configurable: true, value: true })
    const before = editor.getJSON()
    editor.view.dom.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, isComposing: true }),
    )
    editor.view.dom.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true, isComposing: true }),
    )
    expect(editor.getJSON()).toEqual(before)
    expect(openLink).not.toHaveBeenCalled()
    Object.defineProperty(editor.view, 'composing', { configurable: true, value: false })
    editor.view.dispatch(editor.state.tr.setMeta(documentWhitespaceKey, true))
    expect(editor.state.doc.textContent).toBe('にほん')
  },
)
it('le contexte et les liens passent par la fabrique, indépendamment de la vue', async () => {
  const openLink = vi.fn(),
    openContext = vi.fn(() => true)
  const editor = await create('rich-text', {
    content: '<p><a href="https://example.org">texte</a></p>',
    openLink,
  })
  editor.commands.setTextSelection(3)
  const key = new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, cancelable: true })
  editor.view.someProp('handleKeyDown', (handler) => handler(editor.view, key))
  expect(openLink).toHaveBeenCalledWith(editor)
  const body = await create('document', { content: '<p>Texte</p>', openContext })
  body.view.someProp('handleKeyDown', (handler) =>
    handler(
      body.view,
      new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, cancelable: true }),
    ),
  )
  expect(openContext).toHaveBeenCalledWith(body)
})
it('fermer et recréer une zone riche conserve l’historique du document et annule uniquement le champ', async () => {
  const doc = new Y.Doc()
  docs.push(doc)
  const settings = doc.getMap('settings'),
    fragment = new Y.XmlFragment()
  settings.set('abstract', fragment)
  prosemirrorJSONToYXmlFragment(
    getSchema(richZoneExtensions('rich-text')),
    rich('Résumé'),
    fragment,
  )
  const owner = await create('document', { binding: { kind: 'document', document: doc } })
  const history = yUndoPluginKey.getState(owner.state)!.undoManager as Y.UndoManager
  const beforeMount = Y.encodeStateVector(doc)
  const field = await create('rich-text', {
    binding: { kind: 'fragment', fragment, historyOwner: owner },
  })
  expect(Y.encodeStateVector(doc)).toEqual(beforeMount)
  const body = owner.getJSON()
  history.clear()
  field.commands.setTextSelection(7)
  field.commands.insertContent(' enrichi')
  expect(field.state.doc.textContent).toBe('Résumé enrichi')
  field.destroy()
  editors.splice(editors.indexOf(field), 1)
  const reopened = await create('rich-text', {
    binding: { kind: 'fragment', fragment, historyOwner: owner },
  })
  reopened.view.dom.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true, cancelable: true }),
  )
  expect(reopened.state.doc.textContent).toBe('Résumé')
  expect(reopened.can().redo()).toBe(true)
  expect(owner.getJSON()).toEqual(body)
  reopened.view.dom.dispatchEvent(
    new KeyboardEvent('keydown', {
      key: 'z',
      ctrlKey: true,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    }),
  )
  expect(reopened.state.doc.textContent).toBe('Résumé enrichi')
})

it('le code de production ne construit aucune zone en dehors de la fabrique', () => {
  const root = resolve(process.cwd(), 'src')
  const sources = readdirSync(root, { recursive: true }).filter(
    (name): name is string => typeof name === 'string' && name.endsWith('.ts'),
  )
  const constructors = sources.filter((name) =>
    /new\s+Editor\s*\(/.test(readFileSync(join(root, name), 'utf8')),
  )
  const assemblies = sources.filter((name) =>
    /StarterKit\.configure\(/.test(readFileSync(join(root, name), 'utf8')),
  )
  expect(constructors).toEqual(['rich-zone.ts'])
  expect(assemblies).toEqual(['rich-zone.ts'])
})
it('une zone partagée refuse un propriétaire d’historique appartenant à un autre document', async () => {
  const a = new Y.Doc(),
    b = new Y.Doc()
  docs.push(a, b)
  const owner = await create('document', { binding: { kind: 'document', document: a } })
  expect(() =>
    createRichZone({
      profile: 'rich-text',
      label: 'Invalide',
      binding: { kind: 'fragment', fragment: b.getXmlFragment('field'), historyOwner: owner },
    }),
  ).toThrow('same Y.Doc')
})

it('deux zones riches montées fusionnent leurs saisies et chaque historique préserve le texte distant', async () => {
  const a = new Y.Doc(),
    b = new Y.Doc()
  docs.push(a, b)
  const fragmentA = a.getXmlFragment('summary')
  prosemirrorJSONToYXmlFragment(
    getSchema(richZoneExtensions('rich-text')),
    rich('Résumé'),
    fragmentA,
  )
  const ownerA = await create('document', { binding: { kind: 'document', document: a } })
  Y.applyUpdate(b, Y.encodeStateAsUpdate(a), 'remote-test')
  const ownerB = await create('document', { binding: { kind: 'document', document: b } })
  const fieldA = await create('rich-text', {
    binding: { kind: 'fragment', fragment: fragmentA, historyOwner: ownerA },
  })
  const fieldB = await create('rich-text', {
    binding: { kind: 'fragment', fragment: b.getXmlFragment('summary'), historyOwner: ownerB },
  })
  for (const owner of [ownerA, ownerB]) yUndoPluginKey.getState(owner.state)!.undoManager.clear()
  fieldA.commands.insertContentAt(1, 'Premier ')
  fieldB.commands.insertContentAt(7, ' étudié')
  const sync = () => {
    Y.applyUpdate(a, Y.encodeStateAsUpdate(b), 'remote-test')
    Y.applyUpdate(b, Y.encodeStateAsUpdate(a), 'remote-test')
  }
  sync()
  await vi.waitFor(() => {
    expect(fieldA.state.doc.textContent).toBe('Premier Résumé étudié')
    expect(fieldB.getJSON()).toEqual(fieldA.getJSON())
  })
  expect(fieldA.commands.undo()).toBe(true)
  sync()
  await vi.waitFor(() => {
    expect(fieldA.state.doc.textContent).toBe('Résumé étudié')
    expect(fieldB.getJSON()).toEqual(fieldA.getJSON())
  })
  expect(fieldB.commands.undo()).toBe(true)
  sync()
  await vi.waitFor(() => {
    expect(fieldA.state.doc.textContent).toBe('Résumé')
    expect(fieldB.getJSON()).toEqual(fieldA.getJSON())
  })
})

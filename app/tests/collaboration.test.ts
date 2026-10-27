import { afterEach, describe, expect, it } from 'vitest'
import { Editor, getSchema } from '@tiptap/core'
import { prosemirrorJSONToYDoc, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { baseExtensions, collaborativeExtensions } from '../src/extensions'
import {
  Provider,
  stableJSON,
  ACCEPTED_PROPOSAL,
  type Bootstrap,
  type Transport,
} from '../src/provider'
import { prepareProposal } from '../src/assistance'
import fixture from '../fixture.json'

class Host {
  updates: number[][] = []
  generation = 'one'
  revision = 0
  initialized = false
  handlers = new Map<string, Map<string, ((data: unknown) => void)[]>>()
  transport(id: string): Transport {
    return {
      invoke: async <T>(command: string, args?: Record<string, unknown>) => {
        if (command === 'join') {
          const data: Bootstrap = {
            generation: this.generation,
            revision: this.revision,
            updates: structuredClone(this.updates),
            fixture,
            initialize: !this.initialized,
            resources: [],
            restoredDocument: null,
            sessionPath: 'test',
          }
          this.initialized = true
          return data as T
        }
        if (command === 'relay') {
          const update = args!.update as number[]
          this.updates.push([...update])
          this.revision++
          this.emit('web-editor:change', { from: id, revision: this.revision, update: [...update] })
          return this.revision as T
        }
        if (command === 'awareness')
          this.emit('web-editor:awareness', { from: id, update: args!.update })
        return undefined as T
      },
      listen: async <T>(name: string, handler: (data: T) => void) => {
        if (!this.handlers.has(id)) this.handlers.set(id, new Map())
        const handlers = this.handlers.get(id)!.get(name) ?? []
        handlers.push(handler as (data: unknown) => void)
        this.handlers.get(id)!.set(name, handlers)
        return () => {
          const index = handlers.indexOf(handler as (data: unknown) => void)
          if (index >= 0) handlers.splice(index, 1)
        }
      },
    }
  }
  emit(name: string, data: unknown) {
    for (const listeners of this.handlers.values())
      for (const callback of listeners.get(name) ?? []) callback(structuredClone(data))
  }
}
const cleanup: (() => void)[] = []
afterEach(() => {
  for (const dispose of cleanup.splice(0).reverse()) dispose()
})
async function client(host: Host, id: string) {
  const provider = new Provider(id, host.transport(id))
  cleanup.push(() => provider.destroy())
  const schema = getSchema(baseExtensions(() => ''))
  await provider.start((content) => {
    const seed = prosemirrorJSONToYDoc(schema, content, 'default')
    const update = Y.encodeStateAsUpdate(seed)
    seed.destroy()
    return update
  })
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: await collaborativeExtensions(provider, () => {}),
  })
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  cleanup.push(() => editor.destroy())
  yUndoPluginKey.getState(editor.state)?.undoManager.clear()
  return { provider, editor }
}
function append(editor: Editor, text: string) {
  yUndoPluginKey.getState(editor.state)?.undoManager.stopCapturing()
  editor.commands.insertContentAt(editor.state.doc.content.size, {
    type: 'paragraph',
    content: [{ type: 'text', text }],
  })
}
async function flush(...clients: { provider: Provider }[]) {
  for (const c of clients) await c.provider.flush()
}

describe('deux éditeurs structurés et deux Y.Doc indépendants', () => {
  it('retire les lignes initiales distantes sans perdre les caractères, les positions ni l’annulation locale', async () => {
    const host = new Host(),
      a = await client(host, 'main'),
      b = await client(host, 'peer-1')
    a.editor.commands.setContent('<p>Corps</p>')
    await flush(a, b)
    b.editor.commands.setTextSelection(3)
    const remote = new Y.Doc()
    Y.applyUpdate(remote, Y.encodeStateAsUpdate(a.provider.doc))
    const root = remote.getXmlFragment('default')
    root.insert(0, [new Y.XmlElement('paragraph')])
    ;(root.get(1) as Y.XmlElement).insert(0, [new Y.XmlElement('softBreak')])
    const update = Array.from(Y.encodeStateAsUpdate(remote, Y.encodeStateVector(a.provider.doc)))
    host.updates.push(update)
    host.revision++
    host.emit('web-editor:change', { from: 'external', revision: host.revision, update })
    await new Promise((resolve) => setTimeout(resolve, 10))
    await flush(a, b)
    for (const { editor } of [a, b])
      expect(editor.state.doc.content.content.map((node) => node.textContent)).toEqual([
        'Corps',
        '',
      ])
    expect(b.editor.state.selection.from).toBe(3)
    const manager = yUndoPluginKey.getState(b.editor.state)!.undoManager
    manager.clear()
    b.editor.commands.insertContent('texte ')
    await flush(a, b)
    expect(a.editor.state.doc.firstChild!.textContent).toBe('Cotexte rps')
    expect(b.editor.commands.undo()).toBe(true)
    await flush(a, b)
    for (const { editor } of [a, b]) expect(editor.state.doc.firstChild!.textContent).toBe('Corps')
    remote.destroy()
  })

  it('ne fusionne jamais une ancienne vue hors ligne avec un autre document', async () => {
    const host = new Host(),
      a = await client(host, 'main')
    await a.provider.disconnect()
    a.editor.commands.insertContent('Brouillon conservé localement')
    const before = stableJSON(a.editor.getJSON())
    host.generation = 'two'
    await expect(a.provider.reconnect()).rejects.toThrow('remplacé')
    expect(stableJSON(a.editor.getJSON())).toBe(before)
    expect(a.provider.online).toBe(false)
  })
  it('accepte une proposition sans changer le curseur humain et permet une seule annulation', async () => {
    const host = new Host(),
      a = await client(host, 'main'),
      b = await client(host, 'peer-1')
    await flush(a, b)
    a.editor.commands.setTextSelection(3)
    const before = stableJSON(a.editor.getJSON()),
      next = structuredClone(a.editor.getJSON())
    next.content!.push({
      type: 'paragraph',
      attrs: undefined,
      content: [{ type: 'text', text: 'PROPOSITION VALIDÉE', marks: [] }],
    })
    const manager = yUndoPluginKey.getState(a.editor.state)!.undoManager
    manager.trackedOrigins.add(ACCEPTED_PROPOSAL)
    manager.stopCapturing()
    const update = prepareProposal(a.provider, [next])
    host.updates.push(Array.from(update))
    host.revision++
    host.emit('web-editor:change', {
      from: 'agent',
      acceptedBy: 'main',
      revision: host.revision,
      update: Array.from(update),
    })
    expect(a.editor.getText()).toContain('PROPOSITION VALIDÉE')
    expect(b.editor.getText()).toContain('PROPOSITION VALIDÉE')
    expect(a.editor.state.selection.from).toBe(3)
    expect(a.editor.commands.undo()).toBe(true)
    await flush(a, b)
    expect(stableJSON(a.editor.getJSON())).toBe(before)
    expect(stableJSON(b.editor.getJSON())).toBe(before)
  })
  it('distingue retour simple, nouveau paragraphe et retour forcé', async () => {
    const host = new Host(),
      a = await client(host, 'main')
    a.editor.commands.setContent({
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Bonjour' }] }],
    })
    a.editor.commands.setTextSelection(8)
    expect(a.editor.commands.keyboardShortcut('Enter')).toBe(true)
    expect(a.editor.getJSON().content?.[0].content?.at(-1)?.type).toBe('softBreak')
    a.editor.commands.keyboardShortcut('Enter')
    expect(a.editor.getJSON().content).toHaveLength(2)
    a.editor.commands.insertContent('Suite')
    a.editor.commands.keyboardShortcut('Shift-Enter')
    expect(a.editor.getJSON().content?.[1].content?.at(-1)?.type).toBe('hardBreak')
  })
  it('annule la saisie dans la ligne finale en un seul geste, sans entrée d’historique pour sa recréation', async () => {
    const host = new Host(),
      a = await client(host, 'main')
    a.editor.commands.setContent('<p>Avant</p>')
    const manager = yUndoPluginKey.getState(a.editor.state)!.undoManager
    manager.clear()
    a.editor.commands.setTextSelection(a.editor.state.doc.content.size - 1)
    const before = stableJSON(a.editor.getJSON())
    for (const character of 'Reprise') a.editor.commands.insertContent(character)
    expect(manager.undoStack).toHaveLength(1)
    a.editor.commands.undo()
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(stableJSON(a.editor.getJSON())).toBe(before)
    expect(manager.undoStack).toHaveLength(0)
  })
  it.each(['main', 'peer-1'])(
    'conserve une seule ligne de reprise après deux saisies concurrentes et une annulation par %s',
    async (author) => {
      const host = new Host(),
        a = await client(host, 'main'),
        b = await client(host, 'peer-1')
      a.editor.commands.setContent('<p>Début</p>')
      await flush(a, b)
      await b.provider.disconnect()
      for (const [peer, text] of [
        [a, 'ALPHA'],
        [b, 'BETA'],
      ] as const) {
        yUndoPluginKey.getState(peer.editor.state)?.undoManager.stopCapturing()
        peer.editor.commands.insertContentAt(peer.editor.state.doc.content.size - 1, text)
      }
      await b.provider.reconnect()
      await flush(a, b)
      await new Promise((resolve) => setTimeout(resolve, 10))
      await flush(a, b)
      expect(stableJSON(a.editor.getJSON())).toBe(stableJSON(b.editor.getJSON()))
      expect(a.editor.getText()).toContain('ALPHA')
      expect(a.editor.getText().match(/BETA/g)).toHaveLength(1)
      expect(a.editor.state.doc.lastChild!.content.size).toBe(0)
      expect(
        a.editor.state.doc.child(a.editor.state.doc.childCount - 2).content.size,
      ).toBeGreaterThan(0)

      const undoing = author === 'main' ? a : b
      undoing.editor.commands.undo()
      await new Promise((resolve) => setTimeout(resolve, 10))
      await flush(a, b)
      expect(a.editor.getText()).toContain(author === 'main' ? 'BETA' : 'ALPHA')
      expect(a.editor.getText()).not.toContain(author === 'main' ? 'ALPHA' : 'BETA')
      expect(stableJSON(a.editor.getJSON())).toBe(stableJSON(b.editor.getJSON()))
      expect(host.updates.length).toBeLessThan(20)
    },
  )
  it('initialise une seule fois et conserve tableaux, notes et légendes', async () => {
    const host = new Host(),
      a = await client(host, 'main'),
      b = await client(host, 'peer-1')
    await flush(a, b)
    expect(a.provider.doc).not.toBe(b.provider.doc)
    expect(stableJSON(a.editor.getJSON())).toBe(stableJSON(b.editor.getJSON()))
    expect(a.editor.getText().match(/Madame, Monsieur/g)).toHaveLength(1)
    expect(a.editor.getJSON().content?.some((node) => node.type === 'table')).toBe(true)
    expect(a.editor.getHTML()).toContain('data-footnote')
    expect(a.editor.getHTML()).toContain('<figcaption>')
  })
  it('fusionne des modifications hors ligne et annule seulement le travail local', async () => {
    const host = new Host(),
      a = await client(host, 'main'),
      b = await client(host, 'peer-1')
    await b.provider.disconnect()
    append(a.editor, 'AJOUT CAMILLE')
    await flush(a)
    append(b.editor, 'AJOUT ALEX')
    await b.provider.reconnect()
    await flush(a, b)
    expect(a.editor.getText()).toContain('AJOUT CAMILLE')
    expect(a.editor.getText()).toContain('AJOUT ALEX')
    expect(stableJSON(a.editor.getJSON())).toBe(stableJSON(b.editor.getJSON()))
    b.editor.commands.undo()
    await flush(a, b)
    expect(a.editor.getText()).toContain('AJOUT CAMILLE')
    expect(a.editor.getText()).not.toContain('AJOUT ALEX')
    b.editor.commands.redo()
    await flush(a, b)
    expect(a.editor.getText()).toContain('AJOUT ALEX')
  })
  it('conserve deux modifications concurrentes dans la même cellule et la même légende', async () => {
    const host = new Host(),
      a = await client(host, 'main'),
      b = await client(host, 'peer-1')
    await b.provider.disconnect()
    const position = (editor: Editor, type: string) => {
      let found = -1
      editor.state.doc.descendants((node, pos) => {
        if (found < 0 && node.type.name === type) found = pos + (type === 'figure' ? 1 : 2)
      })
      return found
    }
    for (const [peer, token] of [
      [a, 'éA'],
      [b, 'にほんB'],
    ] as const) {
      peer.editor.commands.insertContentAt(position(peer.editor, 'tableCell'), token)
      peer.editor.commands.insertContentAt(position(peer.editor, 'figure'), token)
    }
    await flush(a)
    await b.provider.reconnect()
    await flush(a, b)
    expect(stableJSON(a.editor.getJSON())).toBe(stableJSON(b.editor.getJSON()))
    expect(a.editor.getText()).toContain('éA')
    expect(a.editor.getText()).toContain('にほんB')
  })
  it('reprend un état binaire après fermeture sans dupliquer le contenu', async () => {
    const host = new Host(),
      a = await client(host, 'main')
    append(a.editor, 'Reprise vérifiée : Noël.')
    await flush(a)
    const expected = stableJSON(a.editor.getJSON())
    const next = new Host()
    next.initialized = true
    next.updates = [Array.from(Y.encodeStateAsUpdate(a.provider.doc))]
    const b = await client(next, 'main')
    expect(stableJSON(b.editor.getJSON())).toBe(expected)
    append(b.editor, 'Nouvelle session')
    b.editor.commands.undo()
    expect(stableJSON(b.editor.getJSON())).toBe(expected)
  })
  it('préserve les annotations et les styles par le presse-papier HTML de l’éditeur', async () => {
    const host = new Host(),
      a = await client(host, 'main')
    const html = a.editor.getHTML()
    const plain = new Editor({ extensions: baseExtensions(() => ''), content: html })
    cleanup.push(() => plain.destroy())
    expect(stableJSON(plain.getJSON())).toBe(stableJSON(a.editor.getJSON()))
  })
})

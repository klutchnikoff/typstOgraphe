import { afterEach, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import { Awareness, encodeAwarenessUpdate } from 'y-protocols/awareness'
import { Provider, type Bootstrap, type Transport } from '../src/provider'
import { LocalPresence, type Synchronization } from '../src/local-presence'

const dispose: (() => void)[] = []
afterEach(() => {
  for (const run of dispose.splice(0).reverse()) run()
  vi.useRealTimers()
})
const status = (enabled = false, version = 1): Synchronization => ({
  generation: 'one',
  enabled,
  version,
  windows: ['main'],
})
function host() {
  let current = status()
  const calls: { command: string; args?: Record<string, unknown> }[] = []
  const handlers = new Map<string, Set<(payload: any) => void>>()
  const emit = (name: string, payload: unknown) => {
    for (const handler of handlers.get(name) || []) handler(payload)
  }
  const transport: Transport = {
    async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      calls.push({ command, args })
      if (command === 'join')
        return {
          generation: 'one',
          synchronization: current,
          revision: 0,
          initialize: false,
          resources: [],
          updates: [],
          fixture: { type: 'doc', content: [{ type: 'paragraph' }] },
          restoredDocument: null,
          sessionPath: '',
        } satisfies Bootstrap as T
      if (command === 'relay') return 1 as T
      if (command === 'set_synchronization')
        current = { ...current, enabled: !!args?.enabled, version: current.version + 1 }
      return current as T
    },
    async listen<T>(name: string, handler: (payload: T) => void) {
      if (!handlers.has(name)) handlers.set(name, new Set())
      handlers.get(name)!.add(handler)
      return () => {
        handlers.get(name)!.delete(handler)
      }
    },
  }
  return { transport, calls, handlers, emit }
}

it('ne crée ni heartbeat ni abonnement de présence en individuel, puis libère les deux à l’arrêt', async () => {
  vi.useFakeTimers()
  const doc = new Y.Doc(),
    h = host()
  const presence = new LocalPresence(doc, 'main', h.transport, (error) => {
    throw error
  })
  dispose.push(
    () => doc.destroy(),
    () => presence.destroy(),
  )
  presence.configure(status(), true)
  expect(presence.awareness).toBeUndefined()
  expect(vi.getTimerCount()).toBe(0)
  expect(h.handlers.get('web-editor:awareness')?.size || 0).toBe(0)
  presence.configure(status(true), true)
  await Promise.resolve()
  const active = presence.awareness
  expect(active).toBeDefined()
  expect(vi.getTimerCount()).toBeGreaterThan(0)
  expect(h.calls.some((call) => call.command === 'awareness')).toBe(true)
  await vi.advanceTimersByTimeAsync(32000)
  presence.configure(status(false, 2), true)
  const stopped = h.calls.length
  expect(presence.awareness).toBeUndefined()
  expect(vi.getTimerCount()).toBe(0)
  expect(h.handlers.get('web-editor:awareness')?.size).toBe(0)
  await vi.advanceTimersByTimeAsync(32000)
  expect(h.calls.length).toBe(stopped)
  presence.configure(status(true, 3), true)
  expect(presence.awareness).not.toBe(active)
  expect(doc.isDestroyed).toBe(false)
})

it('ignore les paquets d’une autre session ou période et retire un participant qui part', async () => {
  const doc = new Y.Doc(),
    remote = new Y.Doc(),
    awareness = new Awareness(remote),
    h = host()
  const presence = new LocalPresence(doc, 'main', h.transport, (error) => {
    throw error
  })
  dispose.push(
    () => doc.destroy(),
    () => remote.destroy(),
    () => awareness.destroy(),
    () => presence.destroy(),
  )
  awareness.setLocalState({ window: 'peer', user: { name: 'Collègue' } })
  presence.configure({ ...status(true, 4), windows: ['main', 'peer'] }, true)
  await Promise.resolve()
  const event = {
    generation: 'one',
    version: 4,
    from: 'peer',
    update: Array.from(encodeAwarenessUpdate(awareness, [remote.clientID])),
  }
  h.emit('web-editor:awareness', { ...event, generation: 'old' })
  h.emit('web-editor:awareness', { ...event, version: 3 })
  expect(presence.awareness!.getStates().size).toBe(1)
  h.emit('web-editor:awareness', event)
  expect(presence.awareness!.getStates().size).toBe(2)
  presence.configure(status(true, 5), true)
  expect(presence.awareness!.getStates().size).toBe(1)
})

it('libère un abonnement asynchrone arrivé après l’arrêt sans réactiver le service', async () => {
  const doc = new Y.Doc(),
    h = host(),
    release = vi.fn()
  let subscribed!: (release: () => void) => void
  h.transport.listen = () =>
    new Promise((resolve) => {
      subscribed = resolve
    })
  const presence = new LocalPresence(doc, 'main', h.transport, vi.fn())
  dispose.push(
    () => doc.destroy(),
    () => presence.destroy(),
  )
  presence.configure(status(true), true)
  presence.stop()
  subscribed(release)
  await Promise.resolve()
  expect(release).toHaveBeenCalledOnce()
  expect(presence.awareness).toBeUndefined()
})

it('mémorise le clic pendant une composition et confirme le texte avant d’activer, sans remplacer Y.Doc', async () => {
  const h = host(),
    provider = new Provider('main', h.transport)
  dispose.push(() => provider.destroy())
  await provider.start(() => new Uint8Array([0, 0]))
  const callStart = h.calls.length
  const doc = provider.doc,
    token = provider.beginComposition()
  provider.doc.getMap('test').set('preedit', '日本')
  const activation = provider.setSynchronization(true)
  await Promise.resolve()
  expect(h.calls.some((c) => c.command === 'set_synchronization')).toBe(false)
  expect(provider.awareness).toBeUndefined()
  provider.endComposition(token)
  await activation
  expect(provider.doc).toBe(doc)
  expect(provider.doc.getMap('test').get('preedit')).toBe('日本')
  const enable = h.calls.findIndex((c) => c.command === 'set_synchronization')
  expect(h.calls.slice(callStart).some((c) => c.command === 'relay')).toBe(true)
  expect(h.calls.map((c) => c.command).lastIndexOf('relay')).toBeLessThan(enable)
  expect(provider.synchronization?.enabled).toBe(true)
  expect(provider.awareness).toBeDefined()
})

it('une activation échouée garde le travail et un brouillon refusé ne peut pas activer les services', async () => {
  const h = host(),
    provider = new Provider('main', h.transport)
  dispose.push(() => provider.destroy())
  await provider.start(() => new Uint8Array([0, 0]))
  provider.doc.getMap('test').set('text', 'Travail conservé')
  const invoke = h.transport.invoke
  h.transport.invoke = (command, args) =>
    command === 'set_synchronization' ? Promise.reject('échec') : invoke(command, args)
  await expect(provider.setSynchronization(true)).rejects.toBe('échec')
  expect(provider.doc.getMap('test').get('text')).toBe('Travail conservé')
  expect(provider.synchronization?.enabled).toBe(false)
  expect(provider.synchronizationBusy).toBe(false)
  provider.rejected = { error: 'refus', generation: 'one' }
  const before = h.calls.length
  await provider.setSynchronization(true)
  expect(h.calls.length).toBe(before)
})

it('une fermeture pendant la composition annule l’activation en attente et une ancienne réponse ne change pas le mode', async () => {
  const h = host(),
    provider = new Provider('main', h.transport)
  await provider.start(() => new Uint8Array([0, 0]))
  h.emit('web-editor:synchronization', status(true, 4))
  h.emit('web-editor:synchronization', status(false, 3))
  h.emit('web-editor:synchronization', { ...status(false, 6), generation: 'old' })
  expect(provider.synchronization?.enabled).toBe(true)
  provider.beginComposition()
  const activation = provider.setSynchronization(false)
  provider.destroy()
  await activation
  expect(h.calls.some((c) => c.command === 'set_synchronization')).toBe(false)
  expect(provider.awareness).toBeUndefined()
})

it('suspend la publication pendant la composition sans démonter le service déjà monté', async () => {
  const doc = new Y.Doc(),
    h = host()
  const presence = new LocalPresence(doc, 'main', h.transport, vi.fn())
  dispose.push(
    () => doc.destroy(),
    () => presence.destroy(),
  )
  presence.configure(status(true), true)
  await Promise.resolve()
  const awareness = presence.awareness!
  presence.configure(status(true), true, true)
  const before = h.calls.length
  awareness.setLocalStateField('cursor', { anchor: 'private-preedit' })
  expect(h.calls.length).toBe(before)
  expect(presence.awareness).toBe(awareness)
  presence.configure(status(true), true)
  expect(h.calls.length).toBeGreaterThan(before)
  presence.stop()
  presence.configure(status(true), true, true)
  expect(presence.awareness).toBeUndefined()
})

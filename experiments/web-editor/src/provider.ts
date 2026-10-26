import * as Y from 'yjs'
import {
  Awareness,
  applyAwarenessUpdate,
  encodeAwarenessUpdate,
  removeAwarenessStates,
} from 'y-protocols/awareness'
import type { JSONContent } from '@tiptap/core'

export interface Resource {
  path: string
  data: string
}
export interface Bootstrap {
  revision: number
  updates: number[][]
  fixture: JSONContent
  initialize: boolean
  resources: Resource[]
  restoredDocument: JSONContent | null
  sessionPath: string
}
export interface Transport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>
  listen<T>(name: string, handler: (payload: T) => void): Promise<() => void>
}
const REMOTE = 'remote-relay'
const BOOTSTRAP = 'bootstrap'
interface Change {
  from: string
  revision: number
  update: number[]
}

/** Deux Y.Doc indépendants ; le relais ne partage jamais d'objet JavaScript. */
export class Provider {
  readonly doc = new Y.Doc()
  readonly awareness = new Awareness(this.doc)
  readonly resources = new Map<string, Resource>()
  revision = 0
  online = false
  sessionPath = ''
  onChange = () => {}
  onResource = () => {}
  onError = (_error: unknown) => {}
  private pending: Promise<unknown> = Promise.resolve()
  private disposers: (() => void)[] = []
  private joining = true
  private incoming: Change[] = []

  constructor(
    readonly id: string,
    readonly transport: Transport,
  ) {
    this.doc.on('update', (update: Uint8Array, origin: unknown) => {
      if (origin !== REMOTE && origin !== BOOTSTRAP && this.online) this.send(update)
      this.onChange()
    })
    this.awareness.on(
      'update',
      (
        { added, updated, removed }: { added: number[]; updated: number[]; removed: number[] },
        origin: unknown,
      ) => {
        if (origin !== REMOTE && this.online) this.sendAwareness([...added, ...updated, ...removed])
        if (origin === REMOTE && added.length && this.online)
          this.sendAwareness([this.doc.clientID])
      },
    )
  }

  async start(seed: (fixture: JSONContent) => Uint8Array): Promise<Bootstrap> {
    this.disposers.push(
      await this.transport.listen<Change>('web-editor:change', (event) => {
        if (this.joining) this.incoming.push(event)
        else if (this.online) this.apply(event)
      }),
    )
    this.disposers.push(
      await this.transport.listen<{ from: string; update: number[] }>(
        'web-editor:awareness',
        (event) => {
          if (this.online && event.from !== this.id)
            applyAwarenessUpdate(this.awareness, new Uint8Array(event.update), REMOTE)
        },
      ),
    )
    this.disposers.push(
      await this.transport.listen<Resource>('web-editor:asset', (resource) =>
        this.addResource(resource),
      ),
    )
    const bootstrap = await this.transport.invoke<Bootstrap>('join')
    this.load(bootstrap)
    if (bootstrap.initialize) Y.applyUpdate(this.doc, seed(bootstrap.fixture), BOOTSTRAP)
    this.online = true
    this.finishJoin()
    this.send(Y.encodeStateAsUpdate(this.doc))
    await this.flush()
    return bootstrap
  }

  private apply(event: Change) {
    this.revision = Math.max(this.revision, event.revision)
    if (event.from !== this.id) Y.applyUpdate(this.doc, new Uint8Array(event.update), REMOTE)
  }
  private finishJoin() {
    this.joining = false
    for (const event of this.incoming) this.apply(event)
    this.incoming = []
  }

  private load(bootstrap: Bootstrap) {
    this.revision = Math.max(this.revision, bootstrap.revision)
    this.sessionPath = bootstrap.sessionPath
    for (const resource of bootstrap.resources) this.addResource(resource)
    for (const update of bootstrap.updates) Y.applyUpdate(this.doc, new Uint8Array(update), REMOTE)
  }
  private send(update: Uint8Array) {
    this.pending = this.pending
      .then(async () => {
        const revision = await this.transport.invoke<number>('relay', {
          update: Array.from(update),
        })
        this.revision = Math.max(this.revision, revision)
      })
      .catch((error) => {
        this.online = false
        this.onError(error)
      })
  }
  private sendAwareness(clients: number[]) {
    void this.transport
      .invoke('awareness', { update: Array.from(encodeAwarenessUpdate(this.awareness, clients)) })
      .catch(this.onError)
  }
  addResource(resource: Resource) {
    this.resources.set(resource.path, resource)
    this.onResource()
  }
  imageURL(path: string): string {
    const resource = this.resources.get(path)
    if (!resource) return ''
    const mime = path.endsWith('.svg')
      ? 'image/svg+xml'
      : path.endsWith('.png')
        ? 'image/png'
        : 'image/jpeg'
    return `data:${mime};base64,${resource.data}`
  }
  async flush() {
    await this.pending
    if (!this.online) throw new Error('La fenêtre est déconnectée.')
  }
  async disconnect() {
    await this.flush()
    const own = this.awareness.getLocalState()
    this.awareness.setLocalState(null)
    this.online = false
    this.awareness.setLocalState(own)
    removeAwarenessStates(
      this.awareness,
      [...this.awareness.getStates().keys()].filter((id) => id !== this.doc.clientID),
      REMOTE,
    )
    this.onChange()
  }
  async reconnect() {
    this.joining = true
    const bootstrap = await this.transport.invoke<Bootstrap>('join')
    this.load(bootstrap)
    this.online = true
    this.finishJoin()
    this.send(Y.encodeStateAsUpdate(this.doc))
    await this.flush()
    this.sendAwareness([this.doc.clientID])
    this.onChange()
  }
  destroy() {
    this.online = false
    for (const dispose of this.disposers) dispose()
    this.awareness.destroy()
    this.doc.destroy()
  }
}

export function stableJSON(value: unknown): string {
  return JSON.stringify(value, (_key, item) =>
    item && !Array.isArray(item) && typeof item === 'object'
      ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b)))
      : item,
  )
}

import * as Y from 'yjs'
import {
  Awareness,
  applyAwarenessUpdate,
  encodeAwarenessUpdate,
  removeAwarenessStates,
} from 'y-protocols/awareness'
import type { JSONContent } from '@tiptap/core'
import type { Settings } from './state'
import { tr } from './i18n'

export interface Resource {
  path: string
  data: string
}
export interface Bootstrap {
  nativeEditing?: boolean
  generation?: string
  revision: number
  updates: number[][]
  fixture: JSONContent
  initialize: boolean
  resources: Resource[]
  restoredDocument: JSONContent | null
  sessionPath: string
  settings?: Settings
  hasRecovery?: boolean
  opened?: boolean
  filePath?: string | null
  initialPath?: string | null
}
export interface Transport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>
  listen<T>(name: string, handler: (payload: T) => void): Promise<() => void>
}
const REMOTE = 'remote-relay'
const BOOTSTRAP = 'bootstrap'
export const CONFIRMED_COMMAND = { confirmedCommand: true }
export const ACCEPTED_PROPOSAL = { acceptedProposal: true }
interface Change {
  generation?: string
  from: string
  revision: number
  update: number[]
  acceptedBy?: string
}

export interface HistoryState {
  undo_steps: number
  redo_steps: number
  pending: boolean
}
interface Receipt {
  revision: number
  update: number[]
  history: HistoryState
}
const CONFIRMED_HISTORY = { confirmedHistory: true }

/** Deux Y.Doc indépendants ; le relais ne partage jamais d'objet JavaScript. */
export class Provider {
  readonly doc = new Y.Doc()
  readonly awareness = new Awareness(this.doc)
  readonly resources = new Map<string, Resource>()
  revision = 0
  online = false
  sessionPath = ''
  generation = ''
  onChange = () => {}
  onStatus = () => {}
  documentEdits?: { flush: () => Promise<void>; dirty: () => boolean; draft?: () => unknown }
  onResource = () => {}
  onError = (_error: unknown) => {}
  nativeEditing = false
  historyState: HistoryState = { undo_steps: 0, redo_steps: 0, pending: false }
  historyBusy = false
  composing = false
  private compositionToken = 0
  onHistoryLock = (_locked: boolean) => {}
  rejected: { error: unknown; generation: string } | null = null
  private group = 0
  private lastEdit = 0
  private buffered: { update: Uint8Array; capture: boolean }[] = []
  private scheduled = false
  private queued = 0
  private epoch = 0
  private pending: Promise<unknown> = Promise.resolve()
  private disposers: (() => void)[] = []
  private joining = true
  private incoming: Change[] = []
  private incomingResources: (Resource & { generation?: string })[] = []

  constructor(
    readonly id: string,
    readonly transport: Transport,
  ) {
    this.doc.on(
      'update',
      (update: Uint8Array, origin: unknown, _doc: Y.Doc, transaction: Y.Transaction) => {
        if (
          origin !== REMOTE &&
          origin !== BOOTSTRAP &&
          origin !== ACCEPTED_PROPOSAL &&
          origin !== CONFIRMED_COMMAND &&
          origin !== CONFIRMED_HISTORY &&
          this.online
        )
          this.buffer(update, transaction.meta.get('addToHistory') !== false)
        this.onChange()
      },
    )
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

  async start(
    seed: (fixture: JSONContent) => Uint8Array,
    initialize?: (bootstrap: Bootstrap) => void,
  ): Promise<Bootstrap> {
    this.disposers.push(
      await this.transport.listen<Change>('web-editor:change', (event) => {
        if (this.joining || this.composing) this.incoming.push(event)
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
      await this.transport.listen<Resource & { generation?: string }>(
        'web-editor:asset',
        (resource) => {
          if (this.joining) this.incomingResources.push(resource)
          else if (!resource.generation || resource.generation === this.generation)
            this.addResource(resource)
        },
      ),
    )
    const bootstrap = await this.transport.invoke<Bootstrap>('join')
    this.load(bootstrap)
    if (bootstrap.initialize) Y.applyUpdate(this.doc, seed(bootstrap.fixture), BOOTSTRAP)
    initialize?.(bootstrap)
    this.online = true
    this.finishJoin()
    this.send(Y.encodeStateAsUpdate(this.doc))
    await this.flush()
    if (this.generation)
      await this.transport.invoke('connection', { online: true, generation: this.generation })
    return bootstrap
  }

  private apply(event: Change) {
    if (event.generation && event.generation !== this.generation) return
    this.revision = Math.max(this.revision, event.revision)
    if (event.from !== this.id)
      Y.applyUpdate(
        this.doc,
        new Uint8Array(event.update),
        event.acceptedBy === this.id ? ACCEPTED_PROPOSAL : REMOTE,
      )
    if (event.acceptedBy === this.id && this.nativeEditing)
      void this.refreshHistory().catch(this.onError)
  }
  private finishJoin() {
    this.joining = false
    for (const resource of this.incomingResources)
      if (!resource.generation || resource.generation === this.generation)
        this.addResource(resource)
    this.incomingResources = []
    for (const event of this.incoming) this.apply(event)
    this.incoming = []
  }

  private load(bootstrap: Bootstrap) {
    this.revision = Math.max(this.revision, bootstrap.revision)
    this.nativeEditing = !!bootstrap.nativeEditing
    this.sessionPath = bootstrap.sessionPath
    this.generation = bootstrap.generation || ''
    for (const resource of bootstrap.resources) this.addResource(resource)
    for (const update of bootstrap.updates) Y.applyUpdate(this.doc, new Uint8Array(update), REMOTE)
  }
  stopCapturing() {
    this.drain()
    this.group++
    this.lastEdit = 0
  }
  beginComposition() {
    this.stopCapturing()
    this.composing = true
    return ++this.compositionToken
  }
  endComposition(token = this.compositionToken) {
    if (!this.composing || token !== this.compositionToken) return
    this.composing = false
    if (this.online) this.finishJoin()
    else this.incoming = []
    this.drain()
    this.stopCapturing()
    this.onChange()
  }
  private buffer(update: Uint8Array, capture: boolean) {
    // A single editor dispatch can also repair whitespace. Validate that final
    // synchronous state, never an intermediate transaction missing its repair.
    this.buffered.push({ update, capture })
    if (this.composing) return
    if (this.scheduled) return
    this.scheduled = true
    queueMicrotask(() => this.drain())
  }
  private drain() {
    this.scheduled = false
    if (this.composing) return
    if (!this.buffered.length) return
    const update = Y.mergeUpdates(this.buffered.map((item) => item.update))
    const capture = this.buffered.some((item) => item.capture)
    this.buffered = []
    this.send(update, capture)
  }
  private send(update: Uint8Array, capture = true) {
    if (!this.online || this.rejected) return
    const now = Date.now()
    if (now - this.lastEdit >= 500) this.group++
    this.lastEdit = now
    const group = this.group,
      generation = this.generation,
      epoch = this.epoch
    this.queued++
    this.pending = this.pending
      .then(async () => {
        if (!this.online || this.rejected || this.epoch !== epoch) return
        const receipt = await this.transport.invoke<number | Receipt>('relay', {
          update: Array.from(update),
          generation,
          group,
          capture,
        })
        if (generation !== this.generation || this.epoch !== epoch) return
        if (typeof receipt === 'number') this.revision = Math.max(this.revision, receipt)
        else this.acknowledge(receipt)
      })
      .catch((error) => {
        if (epoch === this.epoch) this.refuse(error, generation)
      })
      .finally(() => {
        this.queued--
        this.onStatus()
      })
  }
  private acknowledge(receipt: Receipt) {
    this.revision = Math.max(this.revision, receipt.revision)
    this.historyState = receipt.history
  }
  private refuse(error: unknown, generation: string) {
    this.online = false
    this.buffered = [] // The complete local replica remains intact in doc/draft().
    this.rejected = { error, generation }
    this.epoch++ // Already queued callbacks may never submit dependent edits.
    void this.transport.invoke('connection', { online: false, generation }).catch(() => {})
    this.onError(error)
    this.onStatus()
  }
  canHistory(redo = false) {
    return (
      this.online &&
      !this.rejected &&
      !this.historyBusy &&
      !this.composing &&
      (redo
        ? this.historyState.redo_steps > 0
        : this.historyState.undo_steps > 0 ||
          this.queued > 0 ||
          this.buffered.length > 0 ||
          !!this.documentEdits?.dirty())
    )
  }
  /** Final typed publication shares the relay/history queue. Snapshots wait for
   * its local installation; all bound zones are locked only during this commit.
   * A refused command is reported to its caller, without quarantining valid input.
   */
  async commitCommand<T>(run: () => Promise<T>, lockEditors = true): Promise<T> {
    if (this.historyBusy || this.composing) throw new Error(tr('Le document a changé. Réessayez.'))
    this.historyBusy = true
    this.onStatus()
    if (lockEditors) this.onHistoryLock(true)
    this.stopCapturing()
    const operation = this.pending.then(run)
    this.pending = operation.then(
      () => {},
      () => {},
    )
    try {
      return await operation
    } finally {
      this.historyBusy = false
      if (lockEditors) this.onHistoryLock(false)
      this.onStatus()
    }
  }
  async history(direction: 'undo' | 'redo') {
    await this.flush()
    if (!this.canHistory(direction === 'redo')) return
    this.historyBusy = true
    this.onStatus()
    this.onHistoryLock(true)
    this.stopCapturing() // Drain synchronous input before appending to the same queue.
    const generation = this.generation,
      epoch = this.epoch
    const operation = this.pending
      .then(async () => {
        if (!this.online || this.rejected || epoch !== this.epoch)
          throw new Error(tr('La synchronisation est suspendue ; votre brouillon est conservé.'))
        const receipt = await this.transport.invoke<Receipt>('editing_history', {
          generation,
          direction,
        })
        if (this.generation !== generation || epoch !== this.epoch)
          throw new Error(tr('Le document a changé. Réessayez.'))
        if (receipt.update.length)
          Y.applyUpdate(this.doc, new Uint8Array(receipt.update), CONFIRMED_HISTORY)
        this.acknowledge(receipt)
      })
      .catch((error) => {
        if (!this.rejected && epoch === this.epoch) this.refuse(error, generation)
      })
    // Snapshots and later repairs wait for the publication and its local installation.
    this.pending = operation
    try {
      await operation
    } finally {
      this.historyBusy = false
      this.onHistoryLock(false)
      this.onStatus()
    }
  }
  async refreshHistory() {
    this.historyState = await this.transport.invoke<HistoryState>('editing_context', {
      generation: this.generation,
    })
    this.onStatus()
  }
  async clearHistory() {
    if (!this.nativeEditing || this.rejected) return
    await this.flush()
    this.historyState = await this.transport.invoke<HistoryState>('reset_editing_context', {
      generation: this.generation,
    })
    this.stopCapturing()
    this.onStatus()
  }
  async discardDraft() {
    if (!this.rejected) return
    // Called only after explicit user confirmation. The next view loads fresh
    // bytes; merging confirmed state into this rejected replica would be unsafe.
    await this.pending
    await this.transport.invoke('reset_editing_context', { generation: this.generation })
    await this.transport.invoke('connection', { online: true, generation: this.generation })
  }
  draft() {
    return {
      format: 'typstographe-rejected-draft-v1',
      generation: this.generation,
      confirmedRevision: this.revision,
      update: Array.from(Y.encodeStateAsUpdate(this.doc)),
      resources: [...this.resources.values()],
      headerDrafts: this.documentEdits?.draft?.(),
    }
  }
  private sendAwareness(clients: number[]) {
    void this.transport
      .invoke('awareness', { update: Array.from(encodeAwarenessUpdate(this.awareness, clients)) })
      .catch(this.onError)
  }
  addResource(resource: Resource) {
    if (this.resources.get(resource.path)?.data === resource.data) return
    this.resources.set(resource.path, { path: resource.path, data: resource.data })
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
  async flush(includeDocumentEdits = true) {
    if (includeDocumentEdits) await this.documentEdits?.flush()
    if (this.composing) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    // Updates can extend the tail while a relay acknowledgment is in flight.
    // A snapshot needs every queued update acknowledged, not just the old tail.
    let tail: Promise<unknown>
    do {
      this.drain()
      tail = this.pending
      await tail
      if (this.composing) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    } while (tail !== this.pending || this.buffered.length > 0)
    if (this.composing) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    if (this.rejected)
      throw new Error(tr('La synchronisation est suspendue ; votre brouillon est conservé.'))
    if (!this.online) throw new Error('La fenêtre est déconnectée.')
  }
  async disconnect() {
    await this.flush()
    const own = this.awareness.getLocalState()
    this.awareness.setLocalState(null)
    if (this.generation)
      await this.transport.invoke('connection', { online: false, generation: this.generation })
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
    if (this.rejected)
      throw new Error(tr('La synchronisation est suspendue ; votre brouillon est conservé.'))
    this.joining = true
    const bootstrap = await this.transport.invoke<Bootstrap>('join')
    if (this.generation && bootstrap.generation !== this.generation) {
      this.joining = false
      throw new Error(
        'Le document a été remplacé dans une autre fenêtre. Rechargez cette vue avant de poursuivre.',
      )
    }
    this.load(bootstrap)
    this.online = true
    this.finishJoin()
    this.send(Y.encodeStateAsUpdate(this.doc))
    await this.flush()
    if (this.generation)
      await this.transport.invoke('connection', { online: true, generation: this.generation })
    this.sendAwareness([this.doc.clientID])
    this.onChange()
  }
  destroy() {
    this.online = false
    this.epoch++
    this.composing = false
    this.compositionToken++
    this.buffered = []
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

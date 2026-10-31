import * as Y from 'yjs'
import {
  Awareness,
  applyAwarenessUpdate,
  encodeAwarenessUpdate,
  removeAwarenessStates,
} from 'y-protocols/awareness'
import type { Transport } from './provider'

export interface Synchronization {
  generation: string
  version: number
  enabled: boolean
  windows: string[]
}
interface PresenceEvent {
  from: string
  generation: string
  version: number
  update: number[]
}
const REMOTE = 'remote-presence'

/** Optional service only. The document, native owner and history outlive it. */
export class LocalPresence {
  awareness?: Awareness
  private user?: { name: string; color: string }
  private paused = false
  private incoming = new Map<string, PresenceEvent>()
  private status?: Synchronization
  private release?: () => void
  private epoch = 0
  private clock = 0
  private observers = new Set<() => void>()
  constructor(
    private doc: Y.Doc,
    private id: string,
    private transport: Transport,
    private error: (error: unknown) => void,
  ) {}
  setUser(user: { name: string; color: string }) {
    this.user = user
    this.awareness?.setLocalStateField('user', user)
  }
  observe(callback: () => void) {
    this.observers.add(callback)
    callback()
    return () => {
      this.observers.delete(callback)
    }
  }
  private changed = () => {
    for (const observer of this.observers) observer()
  }
  configure(status: Synchronization | undefined, ready: boolean, composing = false) {
    this.status = status
    if (!ready || !status?.enabled) {
      this.stop()
      return
    }
    this.paused = composing
    // Keep the mounted cursor plugin during native composition: reconfiguring
    // the view here could disturb its DOM selection. Publish no private preedit.
    if (composing) return
    if (this.awareness) {
      for (const event of this.incoming.values()) this.apply(event)
      this.incoming.clear()
      const states = this.awareness.getStates()
      removeAwarenessStates(
        this.awareness,
        [...states.keys()].filter(
          (id) => id !== this.doc.clientID && !status.windows.includes(states.get(id)?.window),
        ),
        REMOTE,
      )
      this.send([this.doc.clientID])
      this.changed()
      return
    }
    const awareness = (this.awareness = new Awareness(this.doc))
    const epoch = ++this.epoch
    // Awareness clocks belong to the client, not to a particular service instance.
    awareness.meta.get(this.doc.clientID)!.clock = this.clock
    awareness.on('change', this.changed)
    awareness.on(
      'update',
      (
        { added, updated, removed }: { added: number[]; updated: number[]; removed: number[] },
        origin: unknown,
      ) => {
        if (this.awareness !== awareness) return
        if (origin !== REMOTE) this.send([...added, ...updated, ...removed])
        else if (added.length) this.send([this.doc.clientID])
      },
    )
    awareness.setLocalState({
      window: this.id,
      user: this.user || {
        name: this.id === 'main' ? 'Camille' : 'Alex',
        color: this.id === 'main' ? '#286b91' : '#99733b',
      },
    })
    void this.transport
      .listen<PresenceEvent>('web-editor:awareness', (event) => {
        if (this.awareness !== awareness) return
        if (this.paused) this.incoming.set(event.from, event)
        else this.apply(event)
      })
      .then((release) => {
        if (this.epoch !== epoch) release()
        else {
          this.release = release
          this.send([this.doc.clientID])
        }
      })
      .catch((error) => {
        if (this.epoch === epoch) this.error(error)
      })
    this.changed()
  }
  private apply(event: PresenceEvent) {
    const current = this.status
    if (
      !this.awareness ||
      !current ||
      event.generation !== current.generation ||
      event.version !== current.version ||
      event.from === this.id ||
      !current.windows.includes(event.from)
    )
      return
    applyAwarenessUpdate(this.awareness, new Uint8Array(event.update), REMOTE)
  }
  private send(clients: number[]) {
    if (!this.awareness || !this.status || this.paused) return
    const epoch = this.epoch
    const { generation, version } = this.status
    const states = new Map(this.awareness.getStates())
    const own = states.get(this.doc.clientID)
    if (own?.cursor)
      states.set(this.doc.clientID, {
        ...own,
        cursor: {
          ...own.cursor,
          anchor: liveCursorPosition(this.doc, own.cursor.anchor),
          head: liveCursorPosition(this.doc, own.cursor.head),
        },
      })
    void this.transport
      .invoke('awareness', {
        generation,
        version,
        update: Array.from(encodeAwarenessUpdate(this.awareness, clients, states)),
      })
      .catch((error) => {
        if (
          this.epoch === epoch &&
          this.status?.generation === generation &&
          this.status.version === version
        )
          this.error(error)
      })
  }
  stop() {
    this.incoming.clear()
    if (!this.awareness) return
    const awareness = this.awareness
    this.awareness = undefined
    this.epoch++
    this.release?.()
    this.release = undefined
    awareness.off('change', this.changed)
    awareness.destroy() // Includes the library heartbeat and document listener.
    this.clock = awareness.meta.get(this.doc.clientID)!.clock
    this.changed()
  }
  destroy() {
    this.stop()
    this.observers.clear()
  }
}

/** A newly published caret at the start of live text must reference its first
 * live character. A type-boundary anchor may also identify an older cleared
 * boundary in Tiptap’s migration history. Keep that history for persisted anchors.
 * This only changes the affinity of a fresh, transient presence position.
 */
function liveCursorPosition(doc: Y.Doc, position: Y.RelativePosition): Y.RelativePosition {
  if (
    !position ||
    position.item ||
    position.assoc >= 0 ||
    (!position.type && position.tname == null)
  )
    return position
  const absolute = Y.createAbsolutePositionFromRelativePosition(
    Y.createRelativePositionFromJSON(position),
    doc,
  )
  if (absolute?.type instanceof Y.XmlText && absolute.index === 0 && absolute.type.length > 0)
    return Y.createRelativePositionFromTypeIndex(absolute.type, 0, 0)
  return position
}

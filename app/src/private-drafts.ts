/** Private form state belongs to a window, never to its shared document. */
export interface FormDraft {
  id: string
  sourceId?: string
  form: string
  context: unknown
  data: unknown
}
type Entry = FormDraft & { read?: () => unknown; composing: boolean; alive?: () => boolean }
export class PrivateDrafts {
  private entries = new Map<string, Entry>()
  constructor(private notify: () => void) {}
  track(form: string, context: unknown, read: () => unknown, alive?: () => boolean) {
    const entry: Entry = {
      id: crypto.randomUUID(),
      form,
      context: structuredClone(context),
      data: structuredClone(read()),
      read,
      composing: false,
      alive,
    }
    this.entries.set(entry.id, entry)
    this.notify()
    const handle = {
      source: (id: string) => {
        entry.sourceId = id
        this.notify()
      },
      changed: () => {
        if (!entry.composing && this.entries.has(entry.id)) this.notify()
      },
      beginComposition: () => {
        if (!entry.composing) entry.data = structuredClone(read())
        entry.composing = true
      },
      endComposition: () => {
        entry.composing = false
        handle.changed()
      },
      discard: () => {
        if (this.entries.delete(entry.id)) this.notify()
      },
      detach: () => {
        if (!this.entries.has(entry.id)) return
        if (!entry.composing) entry.data = structuredClone(read())
        entry.read = undefined
        entry.alive = undefined
        this.notify()
      },
    }
    return handle
  }
  snapshot(): FormDraft[] {
    return [...this.entries.values()].map((entry) => {
      const value = structuredClone(!entry.composing && entry.read ? entry.read() : entry.data)
      if (entry.alive && !entry.alive()) {
        entry.data = value
        entry.read = undefined
        entry.alive = undefined
      }
      return {
        id: entry.id,
        ...(entry.sourceId ? { sourceId: entry.sourceId } : {}),
        form: entry.form,
        context: structuredClone(entry.context),
        data: value,
      }
    })
  }

  get size() {
    return this.entries.size
  }
}
const owners = new WeakMap<object, PrivateDrafts>()
export const DRAFT_CHANGED = 'private-draft:changed'
export function privateDrafts(owner: object) {
  let drafts = owners.get(owner)
  if (!drafts) {
    drafts = new PrivateDrafts(() =>
      document.dispatchEvent(new CustomEvent(DRAFT_CHANGED, { detail: owner })),
    )
    owners.set(owner, drafts)
  }
  return drafts
}

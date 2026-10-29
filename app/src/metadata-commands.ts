import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import { stableJSON } from './provider'
import { executeStructured, type StructuredCommand } from './structured-commands'
import { tr } from './i18n'

export type FieldTarget =
  | { kind: 'metadata'; field: string }
  | { kind: 'record'; collection: string; id: string; field: string }
type Draft = {
  target: FieldTarget
  value: unknown
  read: () => unknown
  base: string
  version: number
  error?: Error
}
const owners = new WeakMap<Provider, MetadataCommands>()
const key = (target: FieldTarget) => stableJSON(target)

/** Local form values are immediate; only owner-confirmed typed edits enter Yjs.
 * Coalesce our own input, never retry a refused concurrent edit automatically.
 */
export class MetadataCommands {
  private drafts = new Map<string, Draft>()
  private listeners = new Set<() => void>()
  private running?: Promise<void>
  private operations = new Set<Promise<unknown>>()
  private compositions = new Set<HTMLElement>()
  private generation: string
  private identity: unknown
  constructor(
    private editor: Editor,
    private provider: Provider,
  ) {
    this.generation = provider.generation
    this.identity = provider.doc.getMap('settings').get('identity')
    provider.documentEdits = {
      flush: () => this.flush(),
      dirty: () => this.dirty(),
      draft: () => this.draft(),
    }
  }
  private checkDocument() {
    const identity = this.provider.doc.getMap('settings').get('identity')
    if (this.generation !== this.provider.generation || identity !== this.identity) {
      this.drafts.clear()
      this.compositions.clear()
      this.generation = this.provider.generation
      this.identity = identity
    }
  }
  private changed() {
    for (const listener of this.listeners) listener()
    this.provider.onStatus()
    document.dispatchEvent(new Event('metadata:draft'))
  }
  watch(listener: () => void) {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
  value(target: FieldTarget, confirmed: unknown) {
    this.checkDocument()
    return this.drafts.get(key(target))?.value ?? confirmed
  }
  error(target: FieldTarget) {
    return this.drafts.get(key(target))?.error
  }
  draft() {
    this.checkDocument()
    return [...this.drafts.values()].map(({ target, value }) => structuredClone({ target, value }))
  }
  dirty() {
    this.checkDocument()
    return this.drafts.size > 0 || this.compositions.size > 0 || this.operations.size > 0
  }
  composing(input: HTMLElement, active: boolean) {
    if (active) this.compositions.add(input)
    else this.compositions.delete(input)
    this.changed()
  }
  set(target: FieldTarget, value: unknown, read: () => unknown) {
    this.checkDocument()
    const id = key(target),
      previous = this.drafts.get(id)
    if (previous && !previous.error && stableJSON(previous.value) === stableJSON(value)) return
    if (!previous || previous.error)
      this.drafts.set(id, { target, value, read, base: stableJSON(read()), version: 0 })
    else {
      previous.value = value
      previous.version++
    }
    this.changed()
    this.pump()
  }
  discard(target: FieldTarget) {
    this.drafts.delete(key(target))
    this.changed()
  }
  discardAll() {
    this.drafts.clear()
    this.compositions.clear()
    this.changed()
  }
  private pump() {
    if (this.running) return
    const generation = this.provider.generation,
      identity = this.provider.doc.getMap('settings').get('identity')
    this.running = Promise.resolve()
      .then(async () => {
        for (;;) {
          this.checkDocument()
          if (generation !== this.provider.generation || identity !== this.identity) return
          const entry = [...this.drafts.entries()].find(([, draft]) => !draft.error)
          if (!entry) return
          const [id, draft] = entry,
            value = structuredClone(draft.value),
            version = draft.version
          try {
            if (stableJSON(draft.read()) !== draft.base)
              throw new Error(tr('Le document a changé. Réessayez.'))
            await executeStructured(
              this.editor,
              this.provider,
              this.provider.transport,
              [
                {
                  id: 'metadata.set',
                  params: { target: draft.target, value },
                },
              ],
              () => this.drafts.get(id) === draft,
              undefined,
              { guardSelection: false, flushMetadata: false, lockEditors: false },
            )
            if (this.drafts.get(id) !== draft) continue
            if (version === draft.version) this.drafts.delete(id)
            else {
              if (stableJSON(draft.read()) !== stableJSON(value))
                throw new Error(tr('Le document a changé. Réessayez.'))
              draft.base = stableJSON(value)
            }
          } catch (reason) {
            if (this.drafts.get(id) === draft)
              draft.error = reason instanceof Error ? reason : new Error(String(reason))
          }
          this.changed()
        }
      })
      .finally(() => {
        this.running = undefined
        if ([...this.drafts.values()].some((draft) => !draft.error)) this.pump()
      })
  }
  private async flushFields() {
    if (this.compositions.size) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    while (this.running) await this.running
    const refused = [...this.drafts.values()].find((draft) => draft.error)
    if (refused) throw refused.error
    if (this.compositions.size) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
  }
  async flush() {
    this.checkDocument()
    await this.flushFields()
    while (this.operations.size) await Promise.all([...this.operations])
  }
  async run(commands: StructuredCommand[], current: () => boolean = () => true) {
    const operation = (async () => {
      await this.flushFields()
      return executeStructured(
        this.editor,
        this.provider,
        this.provider.transport,
        commands,
        current,
        undefined,
        { guardSelection: false, flushMetadata: false },
      )
    })()
    this.operations.add(operation)
    this.changed()
    try {
      return await operation
    } finally {
      this.operations.delete(operation)
      this.changed()
    }
  }
}
export function metadataCommands(editor: Editor, provider: Provider) {
  let commands = owners.get(provider)
  if (!commands) {
    commands = new MetadataCommands(editor, provider)
    owners.set(provider, commands)
  }
  return commands
}

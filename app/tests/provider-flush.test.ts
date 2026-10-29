import { expect, it } from 'vitest'
import { Provider, type Transport } from '../src/provider'

it('can checkpoint confirmed state while a native field draft awaits a decision', async () => {
  const provider = new Provider('confirmed', { invoke: async () => {} } as unknown as Transport)
  provider.online = true
  provider.documentEdits = {
    dirty: () => true,
    flush: async () => {
      throw new Error('champ refusé')
    },
    draft: () => [{ target: { kind: 'metadata', field: 'letter.subject' }, value: 'À conserver' }],
  }
  try {
    await expect(provider.flush()).rejects.toThrow('champ refusé')
    await expect(provider.flush(false)).resolves.toBeUndefined()
    expect(provider.rejected).toBeNull()
    expect(provider.draft().headerDrafts).toEqual([
      { target: { kind: 'metadata', field: 'letter.subject' }, value: 'À conserver' },
    ])
  } finally {
    provider.destroy()
  }
})

it('a scalar header commit keeps rich zones editable while snapshots still await confirmation', async () => {
  const provider = new Provider('native-field', { invoke: async () => {} } as unknown as Transport)
  provider.online = true
  const locks: boolean[] = []
  provider.onHistoryLock = (value) => locks.push(value)
  let finish!: () => void
  try {
    const operation = provider.commitCommand(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        }),
      false,
    )
    await Promise.resolve()
    let saved = false
    const snapshot = provider.flush().then(() => {
      saved = true
    })
    await Promise.resolve()
    expect(saved).toBe(false)
    expect(locks).toEqual([])
    finish()
    await operation
    await snapshot
    expect(saved).toBe(true)
    expect(locks).toEqual([])
  } finally {
    provider.destroy()
  }
})

it('serializes a typed commit with snapshots and unlocks after a refusal without losing the session', async () => {
  const provider = new Provider('typed', { invoke: async () => {} } as unknown as Transport)
  provider.online = true
  const locks: boolean[] = []
  provider.onHistoryLock = (locked) => locks.push(locked)
  let finish!: () => void
  try {
    const committing = provider.commitCommand(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        }),
    )
    await Promise.resolve()
    let saved = false
    const snapshot = provider.flush().then(() => {
      saved = true
    })
    await Promise.resolve()
    expect(saved).toBe(false)
    expect(provider.historyBusy).toBe(true)
    finish()
    await committing
    await snapshot
    expect(saved).toBe(true)
    await expect(
      provider.commitCommand(async () => {
        throw new Error('stale')
      }),
    ).rejects.toThrow('stale')
    expect(provider.historyBusy).toBe(false)
    expect(provider.rejected).toBeNull()
    expect(provider.online).toBe(true)
    expect(locks).toEqual([true, false, true, false])
  } finally {
    provider.destroy()
  }
})

it('attend aussi les mises à jour ajoutées pendant un accusé de réception', async () => {
  const replies: ((revision: number) => void)[] = []
  const transport = {
    invoke: (command: string) =>
      command === 'relay'
        ? new Promise<number>((resolve) => replies.push(resolve))
        : Promise.resolve(),
  } as unknown as Transport
  const provider = new Provider('test', transport)
  provider.online = true
  try {
    provider.doc.getText('body').insert(0, 'A')
    for (let i = 0; i < 3; i++) await Promise.resolve()
    let complete = false
    const flushed = provider.flush().then(() => {
      complete = true
    })
    provider.doc.getText('body').insert(1, 'B')
    replies[0](1)
    for (let i = 0; i < 8; i++) await Promise.resolve()
    expect(replies).toHaveLength(2)
    expect(complete).toBe(false)
    replies[1](2)
    await flushed
    expect(provider.revision).toBe(2)
    expect(complete).toBe(true)
  } finally {
    provider.destroy()
  }
})

it('conserve toute la frappe après un refus, bloque la file et refuse une reconnexion implicite', async () => {
  let reject!: (reason: unknown) => void
  let relays = 0
  const transport = {
    invoke: (command: string) => {
      if (command === 'relay') {
        relays++
        return new Promise((_, fail) => {
          reject = fail
        })
      }
      return Promise.resolve()
    },
  } as unknown as Transport
  const provider = new Provider('test', transport)
  provider.online = true
  provider.generation = 'document'
  try {
    const text = provider.doc.getText('body')
    text.insert(0, 'Premier')
    for (let i = 0; i < 3; i++) await Promise.resolve()
    text.insert(text.length, ' second')
    provider.stopCapturing()
    reject('invalid_document')
    await expect(provider.flush()).rejects.toThrow('brouillon')
    text.insert(text.length, ' conservé')
    expect(relays).toBe(1)
    expect(text.toString()).toBe('Premier second conservé')
    expect(provider.draft().update.length).toBeGreaterThan(0)
    await expect(provider.reconnect()).rejects.toThrow('brouillon')
    expect(provider.online).toBe(false)
  } finally {
    provider.destroy()
  }
})

it('regroupe les transactions synchrones avant validation, sans délai de frappe', async () => {
  const updates: number[][] = []
  const provider = new Provider('test', {
    invoke: async (command: string, args: Record<string, unknown>) => {
      if (command === 'relay') {
        updates.push(args.update as number[])
        return updates.length
      }
    },
  } as unknown as Transport)
  provider.online = true
  try {
    let contentChanges = 0
    provider.onChange = () => {
      contentChanges++
    }
    provider.doc.getText('body').insert(0, 'A')
    provider.doc.getText('body').insert(1, 'B')
    expect(provider.doc.getText('body').toString()).toBe('AB')
    await provider.flush()
    expect(updates).toHaveLength(1)
    expect(contentChanges).toBe(2) // The acknowledgment never requests another rendering.
    expect(provider.revision).toBe(1)
  } finally {
    provider.destroy()
  }
})

it('ne publie aucune préédition et confirme la composition en un lot', async () => {
  let relays = 0
  const provider = new Provider('test', {
    invoke: async (command: string) => (command === 'relay' ? ++relays : undefined),
  } as unknown as Transport)
  provider.online = true
  try {
    provider.beginComposition()
    provider.doc.getText('body').insert(0, 'にほ')
    for (let i = 0; i < 4; i++) await Promise.resolve()
    expect(relays).toBe(0)
    await expect(provider.flush()).rejects.toThrow('saisie')
    provider.doc.getText('body').insert(2, 'ん')
    provider.endComposition()
    await provider.flush()
    expect(provider.doc.getText('body').toString()).toBe('にほん')
    expect(relays).toBe(1)
  } finally {
    provider.destroy()
  }
})

it('une annulation devenue sans effet ne déconnecte pas la fenêtre', async () => {
  const provider = new Provider('test', {
    invoke: async () => ({
      revision: 0,
      update: [],
      history: { undo_steps: 0, redo_steps: 0, pending: false },
    }),
  } as unknown as Transport)
  provider.online = true
  provider.historyState.undo_steps = 1
  try {
    await provider.history('undo')
    expect(provider.rejected).toBeNull()
    expect(provider.online).toBe(true)
    expect(provider.historyState.undo_steps).toBe(0)
  } finally {
    provider.destroy()
  }
})

it('une composition commencée pendant l’attente d’un accusé interdit aussi l’instantané', async () => {
  let release!: (value: number) => void
  const provider = new Provider('test', {
    invoke: (command: string) =>
      command === 'relay'
        ? new Promise<number>((resolve) => {
            release = resolve
          })
        : Promise.resolve(),
  } as unknown as Transport)
  provider.online = true
  try {
    provider.doc.getText('body').insert(0, 'Avant')
    for (let i = 0; i < 3; i++) await Promise.resolve()
    const flushed = provider.flush()
    provider.beginComposition()
    provider.doc.getText('body').insert(5, 'に')
    release(1)
    await expect(flushed).rejects.toThrow('saisie')
    expect(provider.doc.getText('body').toString()).toBe('Avantに')
  } finally {
    provider.destroy()
  }
})

it('une fin de composition tardive ne publie pas la préédition de la zone suivante', async () => {
  let relays = 0
  const provider = new Provider('test', {
    invoke: async (command: string) => (command === 'relay' ? ++relays : undefined),
  } as unknown as Transport)
  provider.online = true
  try {
    const first = provider.beginComposition()
    provider.doc.getText('body').insert(0, 'A')
    const second = provider.beginComposition()
    provider.doc.getText('body').insert(1, 'B')
    provider.endComposition(first)
    expect(provider.composing).toBe(true)
    expect(relays).toBe(0)
    provider.endComposition(second)
    await provider.flush()
    expect(provider.doc.getText('body').toString()).toBe('AB')
    expect(relays).toBe(1)
  } finally {
    provider.destroy()
  }
})

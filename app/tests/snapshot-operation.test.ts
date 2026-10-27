import { afterEach, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import { draftGuard, withCurrentSnapshot } from '../src/snapshot-operation'

const documents: Y.Doc[] = []
afterEach(() => documents.splice(0).forEach((doc) => doc.destroy()))
function session() {
  const doc = new Y.Doc()
  documents.push(doc)
  doc.getMap('settings').set('identity', 'courrier-one')
  return { doc, generation: 'one', flush: vi.fn(async () => {}) }
}
it('reprend un instantané complet après un relais tardif, en attendant les envois en cours', async () => {
  const provider = session()
  let revision = 1,
    text = 'Avant'
  const persisted: { revision: number; document: string; update: string }[] = []
  const operation = vi.fn(
    async (snapshot: { revision: number; document: string; update: string }) => {
      if (snapshot.revision === 1) {
        revision = 2
        text = 'Après édition distante'
        throw 'stale'
      }
      persisted.push(snapshot)
      return 'courrier.typsto'
    },
  )
  const result = await withCurrentSnapshot(
    provider,
    () => ({ revision, document: text, update: `état-${revision}` }),
    operation,
  )
  expect(result).toBe('courrier.typsto')
  expect(provider.flush).toHaveBeenCalledTimes(2)
  expect(persisted).toEqual([{ revision: 2, document: 'Après édition distante', update: 'état-2' }])
})
it('borne les reprises à trois tentatives et ne masque jamais une autre erreur', async () => {
  const provider = session(),
    operation = vi.fn(async () => {
      throw new Error('stale')
    })
  await expect(withCurrentSnapshot(provider, () => ({}), operation)).rejects.toThrow('stale')
  expect(operation).toHaveBeenCalledTimes(3)
  const failure = new Error('Permission refusée'),
    denied = vi.fn(async () => {
      throw failure
    })
  await expect(withCurrentSnapshot(provider, () => ({}), denied)).rejects.toBe(failure)
  expect(denied).toHaveBeenCalledTimes(1)
})
it.each(['identity', 'generation'])(
  'ne reprend jamais une sauvegarde dans un autre document (%s)',
  async (changed) => {
    const provider = session(),
      operation = vi.fn(async () => {
        if (changed === 'identity') provider.doc.getMap('settings').set('identity', 'courrier-two')
        else provider.generation = 'two'
        throw 'stale'
      })
    await expect(withCurrentSnapshot(provider, () => ({}), operation)).rejects.toThrow(
      'Le document a changé',
    )
    expect(operation).toHaveBeenCalledTimes(1)
  },
)

it('attend ses propres relais avant de figer un brouillon sans accepter une édition concurrente', async () => {
  const provider = { ...session(), revision: 1 }
  provider.flush.mockImplementation(async () => {
    provider.revision++
  })
  const check = await draftGuard(provider, () => true)
  expect(check).not.toThrow()
  provider.revision++
  expect(check).toThrow('Le document a changé')
  expect(provider.flush).toHaveBeenCalledTimes(1)
})
it('refuse un brouillon modifié pendant le drainage ou la validation, sans reprise', async () => {
  const provider = { ...session(), revision: 1 }
  let same = true
  provider.flush.mockImplementation(async () => {
    same = false
  })
  await expect(draftGuard(provider, () => same)).rejects.toThrow('Le document a changé')
  provider.flush.mockImplementation(async () => {})
  same = true
  const check = await draftGuard(provider, () => same)
  same = false
  expect(check).toThrow('Le document a changé')
})
it.each(['identity', 'generation'])(
  'invalide le brouillon après changement de document (%s)',
  async (changed) => {
    const provider = { ...session(), revision: 1 }
    const check = await draftGuard(provider, () => true)
    if (changed === 'identity') provider.doc.getMap('settings').set('identity', 'another')
    else provider.generation = 'another'
    expect(check).toThrow('Le document a changé')
  },
)

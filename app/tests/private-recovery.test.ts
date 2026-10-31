import { afterEach, expect, it, vi } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { Provider, type Transport } from '../src/provider'
import { privateDrafts } from '../src/private-drafts'
import { privateRecovery } from '../src/private-recovery'

const cleanups: (() => void)[] = []
afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup()
  vi.useRealTimers()
})
function setup() {
  const writes: any[] = []
  const invoke = vi.fn(async (command: string, args?: any) => {
    if (command === 'open_private_recovery') return 4
    if (command === 'conserve_private') {
      writes.push(structuredClone(args.input))
      return true
    }
  })
  const provider = new Provider('private', { invoke } as unknown as Transport)
  provider.generation = 'original-generation'
  provider.rejected = { error: 'refused', generation: provider.generation }
  provider.documentEdits = {
    dirty: () => true,
    flush: async () => {
      throw new Error('Unconfirmed field must stay private')
    },
    draft: () => [{ target: { kind: 'metadata', field: 'letter.subject' }, value: 'Pending' }],
  }
  const flush = vi.spyOn(provider, 'flush')
  const editor = new Editor({ extensions: [StarterKit], content: '<p>Local unconfirmed text</p>' })
  const failed = vi.fn()
  const recovery = privateRecovery(editor, provider, () => true, failed)
  cleanups.push(() => {
    recovery.stop()
    editor.destroy()
    provider.destroy()
  })
  return { provider, editor, recovery, flush, failed, writes, invoke }
}
it('protects rejected and offline input without submitting fields, compiling or publishing it', async () => {
  const { provider, recovery, writes, flush, invoke } = setup()
  privateDrafts(provider).track('author', { collection: 'authors' }, () => ({ name: 'Private' }))
  recovery.changed()
  await recovery.flush()
  expect(flush).not.toHaveBeenCalled()
  expect(invoke.mock.calls.map(([command]) => command)).toEqual([
    'open_private_recovery',
    'conserve_private',
  ])
  expect(writes[0]).toMatchObject({
    generation: provider.generation,
    epoch: 4,
    sequence: 1,
    rejected: true,
    fields: [{ value: 'Pending' }],
    forms: [{ data: { name: 'Private' } }],
  })
  expect(JSON.stringify(writes[0].document)).toContain('Local unconfirmed text')
  recovery.changed()
  await recovery.flush()
  expect(writes).toHaveLength(1)
  provider.generation = 'next-document'
  recovery.changed()
  await recovery.flush()
  expect(writes).toHaveLength(1)
})

it('leaves failed input dirty, retries it and never writes after the view is stopped', async () => {
  vi.useFakeTimers()
  const { recovery, invoke, writes, failed } = setup()
  await Promise.resolve()
  invoke.mockResolvedValueOnce(4).mockRejectedValueOnce(new Error('Disk full'))
  recovery.changed()
  await vi.advanceTimersByTimeAsync(2000)
  expect(failed).toHaveBeenCalledTimes(1)
  expect(String(failed.mock.calls[0][0])).toContain('La copie de secours')
  await vi.advanceTimersByTimeAsync(2000)
  expect(writes).toHaveLength(1)
  expect(writes[0].sequence).toBe(2)
  recovery.stop()
  recovery.changed()
  await vi.advanceTimersByTimeAsync(20000)
  expect(writes).toHaveLength(1)
})

it('retries view registration after a transient storage failure', async () => {
  vi.useFakeTimers()
  const { recovery, invoke, writes, failed } = setup()
  invoke.mockRejectedValueOnce(new Error('Storage temporarily unavailable'))
  recovery.changed()
  await vi.advanceTimersByTimeAsync(2000)
  expect(failed).toHaveBeenCalledTimes(1)
  expect(writes).toHaveLength(0)
  await vi.advanceTimersByTimeAsync(2000)
  expect(writes).toHaveLength(1)
  expect(writes[0].epoch).toBe(4)
})

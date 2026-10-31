import { expect, it, vi } from 'vitest'
import { MetadataCommands } from '../src/metadata-commands'
import { Provider, type Transport } from '../src/provider'
import type { Editor } from '@tiptap/core'
import { executeStructured } from '../src/structured-commands'
vi.mock('../src/structured-commands', () => ({ executeStructured: vi.fn() }))
it('a recovered field stays private until explicit confirmation and successful owner acknowledgement', async () => {
  const provider = new Provider('main', { invoke: async () => {} } as unknown as Transport)
  const commands = new MetadataCommands({} as Editor, provider),
    done = vi.fn(async () => {})
  const target = { kind: 'metadata' as const, field: 'letter.subject' }
  let accepted!: () => void
  vi.mocked(executeStructured).mockImplementationOnce(
    () => new Promise<void>((resolve) => (accepted = resolve)) as any,
  )
  commands.restore(target, 'Private subject', () => 'Confirmed subject', done, 'source-id')
  commands.set(target, 'Edited private subject', () => 'Confirmed subject')
  await Promise.resolve()
  expect(executeStructured).not.toHaveBeenCalled()
  expect(commands.recovering(target)).toBe(true)
  expect(commands.draft()).toEqual([
    { target, value: 'Edited private subject', base: 'Confirmed subject', sourceId: 'source-id' },
  ])
  commands.confirm(target, 'Edited private subject', () => 'Confirmed subject')
  await vi.waitFor(() => expect(executeStructured).toHaveBeenCalledOnce())
  expect(done).not.toHaveBeenCalled()
  accepted()
  await commands.flush()
  await vi.waitFor(() => expect(done).toHaveBeenCalledOnce())
  expect(commands.draft()).toEqual([])
  provider.destroy()
})

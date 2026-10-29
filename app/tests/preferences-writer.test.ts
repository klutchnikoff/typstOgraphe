import { expect, it, vi } from 'vitest'
import { preferencesWriter } from '../src/preferences-writer'
import type { Preferences } from '../src/preferences-panel'
import type { Transport } from '../src/provider'

const value = (): Preferences => ({
  appearance: 'system',
  font: 'serif',
  size: 18,
  personal: {},
  profile: {},
  signature: null,
  recent: [],
  editor_fraction: 0.5,
  spelling_enabled: true,
})

it('an older slow write finishes before the newer snapshot and queued snapshots are immutable', async () => {
  let finish!: () => void
  const slow = new Promise<void>((resolve) => (finish = resolve))
  const written: Preferences[] = []
  const invoke = vi.fn(async (_: string, args: { preferences: Preferences }) => {
    if (!written.length) await slow
    written.push(args.preferences)
  })
  const save = preferencesWriter({ invoke } as unknown as Pick<Transport, 'invoke'>)
  const original = value()
  const first = save(original)
  original.appearance = 'dark'
  original.size = 21
  const last = save(original)
  original.profile['sender.name'] = 'Modification après demande'
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1))
  expect(written).toEqual([])
  finish()
  await Promise.all([first, last])
  expect(written.map((p) => [p.appearance, p.size])).toEqual([
    ['system', 18],
    ['dark', 21],
  ])
  expect(written.every((p) => !p.profile['sender.name'])).toBe(true)
})

it('a failed write reaches its caller while a subsequent valid write can still complete', async () => {
  const invoke = vi
    .fn()
    .mockRejectedValueOnce(new Error('disk unavailable'))
    .mockResolvedValueOnce(undefined)
  const save = preferencesWriter({ invoke } as unknown as Pick<Transport, 'invoke'>)
  const failed = save(value())
  const next = save({ ...value(), appearance: 'dark' })
  await expect(failed).rejects.toThrow('disk unavailable')
  await expect(next).resolves.toBeUndefined()
  expect(invoke).toHaveBeenCalledTimes(2)
  expect(invoke.mock.calls[1][1].preferences.appearance).toBe('dark')
})

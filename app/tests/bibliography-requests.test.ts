import { afterEach, expect, it, vi } from 'vitest'
import type { Editor } from '@tiptap/core'
import { Provider, type Transport } from '../src/provider'
import { bibliographyRequests } from '../src/bibliography-requests'
const providers: Provider[] = []
afterEach(() => providers.splice(0).forEach((provider) => provider.destroy()))
function setup() {
  const transport: Transport = { invoke: vi.fn(), listen: vi.fn() }
  const provider = new Provider('test', transport)
  providers.push(provider)
  provider.generation = 'generation-1'
  provider.doc.getMap('settings').set('identity', 'document-1')
  const editor = { isDestroyed: false } as Editor
  return { provider, editor, requests: bibliographyRequests(provider, editor, transport) }
}
it('discards stale search responses independently from import decisions and usage requests', () => {
  const { requests } = setup()
  const first = requests.latest('browse'),
    review = requests.latest('review')
  const second = requests.latest('browse')
  expect(first()).toBe(false)
  expect(second()).toBe(true)
  expect(review()).toBe(true)
  requests.destroy()
  expect(second()).toBe(false)
  expect(review()).toBe(false)
  expect(requests.latest('usage')()).toBe(false)
})
it.each(['generation', 'identity', 'destroyed'] as const)(
  'fences all late responses after %s changes',
  (change) => {
    const { provider, editor, requests } = setup()
    const response = requests.latest('browse')
    if (change === 'generation') provider.generation = 'generation-2'
    if (change === 'identity') provider.doc.getMap('settings').set('identity', 'document-2')
    if (change === 'destroyed') Object.assign(editor, { isDestroyed: true })
    expect(response()).toBe(false)
    expect(requests.current()).toBe(false)
  },
)

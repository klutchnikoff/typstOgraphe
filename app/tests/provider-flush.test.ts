import { expect, it } from 'vitest'
import { Provider, type Transport } from '../src/provider'

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
    await Promise.resolve()
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

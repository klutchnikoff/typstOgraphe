import type { Transport } from './provider'

export async function browserTransport(): Promise<{ id: string; transport: Transport }> {
  const id = new URLSearchParams(location.search).get('client') || 'main'
  const events = new EventSource('/__spike/events')
  await new Promise<void>((resolve, reject) => {
    events.onopen = () => resolve()
    events.onerror = () => reject(new Error('Relais de recette inaccessible.'))
  })
  return {
    id,
    transport: {
      invoke: async <T>(command: string, args: Record<string, unknown> = {}) => {
        if (command === 'open_peer') {
          window.open(`/?client=peer-${Date.now()}`, '_blank')
          return undefined as T
        }
        const response = await fetch('/__spike/invoke', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ command, args, peer: id }),
        })
        if (response.headers.get('Content-Type') === 'application/octet-stream')
          return (await response.arrayBuffer()) as T
        const data = await response.json()
        if (data.error) throw data.error
        return data.result as T
      },
      listen: async <T>(name: string, handler: (data: T) => void) => {
        const listener = (event: MessageEvent) => {
          const data = JSON.parse(event.data)
          if (data.name === name) handler(data.payload)
        }
        events.addEventListener('message', listener)
        return () => events.removeEventListener('message', listener)
      },
    },
  }
}

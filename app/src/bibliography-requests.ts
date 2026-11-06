import type { Editor } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import { bibliographyQuery } from './bibliography-query'
import { documentContent } from './shared-inline-marks'

/** Requests belong to one document generation and one manager lifetime.
 * Independent channels prevent a search from cancelling an import review.
 */
export function bibliographyRequests(provider: Provider, editor: Editor, transport: Transport) {
  const generation = provider.generation
  const identity = provider.doc.getMap('settings').get('identity')
  const channels = new Map<string, object>()
  let alive = true
  const current = () =>
    alive &&
    !editor.isDestroyed &&
    generation === provider.generation &&
    identity === provider.doc.getMap('settings').get('identity')
  return {
    current,
    latest(channel: string) {
      const token = {}
      channels.set(channel, token)
      return () => current() && channels.get(channel) === token
    },
    query<T>(operation: string, params: Record<string, unknown> = {}) {
      return bibliographyQuery<T>(
        provider,
        transport,
        operation,
        params,
        documentContent(editor.getJSON()),
      )
    },
    destroy() {
      alive = false
      channels.clear()
    },
  }
}

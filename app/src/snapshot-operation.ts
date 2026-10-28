import type { Provider } from './provider'
import { tr } from './i18n'

type SnapshotSession = Pick<Provider, 'doc' | 'generation' | 'flush'>

/** Drain our own relay acknowledgements before guarding an asynchronous edit draft.
 * Unlike persistence, a draft never retries or merges a concurrent edit.
 */
export async function draftGuard(
  provider: SnapshotSession & Pick<Provider, 'revision'>,
  unchanged: () => boolean,
  includeDocumentEdits = true,
): Promise<() => void> {
  const generation = provider.generation,
    identity = provider.doc.getMap('settings').get('identity')
  const check = () => {
    if (
      provider.generation !== generation ||
      provider.doc.getMap('settings').get('identity') !== identity ||
      !unchanged()
    )
      throw new Error(tr('Le document a changé. Réessayez.'))
  }
  await provider.flush(includeDocumentEdits)
  check()
  const revision = provider.revision
  return () => {
    check()
    if (provider.revision !== revision) throw new Error(tr('Le document a changé. Réessayez.'))
  }
}

/** Manual persistence follows the current document, while the backend keeps its strict guard. */
export async function withCurrentSnapshot<T, R>(
  provider: SnapshotSession,
  capture: () => T,
  operation: (snapshot: T) => Promise<R>,
): Promise<R> {
  const generation = provider.generation
  const identity = provider.doc.getMap('settings').get('identity')
  for (let attempt = 0; ; attempt++) {
    await provider.flush()
    if (
      provider.generation !== generation ||
      provider.doc.getMap('settings').get('identity') !== identity
    )
      throw new Error(tr('Le document a changé ; recommencez l’opération.'))
    try {
      return await operation(capture())
    } catch (error) {
      const message = error instanceof Error ? error.message : error
      // A late relay/whitespace repair can advance the revision during compilation.
      // Recapture everything together; never re-send an outdated snapshot or retry other errors.
      if (message !== 'stale' || attempt >= 2) throw error
    }
  }
}

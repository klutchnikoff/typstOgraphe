import type { Preferences } from './preferences-panel'
import type { Transport } from './provider'

/** Serialize immutable snapshots from every preference producer in this window. */
export function preferencesWriter(transport: Pick<Transport, 'invoke'>) {
  let pending = Promise.resolve()
  return (preferences: Preferences): Promise<void> => {
    const snapshot = structuredClone(preferences)
    const saved = pending
      .catch(() => {})
      .then(() => transport.invoke<void>('preferences_save', { preferences: snapshot }))
    pending = saved
    return saved
  }
}

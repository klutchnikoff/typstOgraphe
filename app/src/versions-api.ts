import type { Provider, Transport } from './provider'
import type { VersionAPI, VersionCatalogue } from './version-types'
import { comparisonAPI } from './comparison-api'
import { tr } from './i18n'
export function versionsAPI(options: {
  creator?: string | null
  provider: Provider
  transport: Transport
  destination(): Promise<string | null | undefined>
  saved(
    result: VersionCatalogue & { path: string; revision: number; warning?: string },
  ): Promise<void>
}): VersionAPI {
  const generation = options.provider.generation
  async function guard() {
    // Never commit open metadata fields or forms merely to capture a version.
    await options.provider.flush(false)
    if (options.provider.generation !== generation)
      throw new Error(tr('Le document a changé. Réessayez.'))
    return { document: generation, revision: options.provider.revision }
  }
  return {
    comparison: comparisonAPI(options.provider, options.transport),
    async list() {
      return options.transport.invoke('versions_list', { guard: await guard() })
    },
    async read(id) {
      return options.transport.invoke('versions_read', { guard: await guard(), id })
    },
    async preview(id) {
      return options.transport.invoke('versions_preview', { guard: await guard(), id })
    },
    async change(history, action) {
      const destination = await options.destination()
      if (destination === null) return null
      if (action.kind === 'create' && options.creator)
        action = { ...action, details: { ...action.details, creator: options.creator } }
      const result = await options.transport.invoke<
        VersionCatalogue & { path: string; revision: number; warning?: string }
      >('versions_change', { guard: await guard(), change: { history, action }, destination })
      if (options.provider.generation !== generation)
        throw new Error(tr('Le document a changé. Réessayez.'))
      await options.saved(result)
      return result
    },
  }
}

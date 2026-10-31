import * as Y from 'yjs'
import { CONFIRMED_COMMAND, type Provider, type Transport, type Resource } from './provider'
import type { ComparisonAPI } from './comparison-types'
import { tr } from './i18n'
export function comparisonAPI(provider: Provider, transport: Transport): ComparisonAPI {
  const generation = provider.generation
  async function ready() {
    if (provider.composing) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    await provider.flush(false)
    if (provider.generation !== generation) throw new Error(tr('Le document a changé. Réessayez.'))
    return { document: generation, revision: provider.revision }
  }
  return {
    async open(origin) {
      return transport.invoke('comparison_open', { guard: await ready(), origin })
    },
    async read(id) {
      return transport.invoke('comparison_read', { id })
    },
    async decide(id, groups, action) {
      await ready()
      const commit = async () => {
        const result = await transport.invoke<
          Awaited<ReturnType<ComparisonAPI['decide']>> & {
            publication?: {
              guard: { document: string; revision: number }
              update: number[]
              resources: Resource[]
            } | null
          }
        >('comparison_decide', { id, groups, action })
        if (provider.generation !== generation || provider.rejected) return result
        if (result.publication) {
          for (const resource of result.publication.resources) provider.addResource(resource)
          Y.applyUpdate(provider.doc, new Uint8Array(result.publication.update), CONFIRMED_COMMAND)
          provider.revision = Math.max(provider.revision, result.publication.guard.revision)
          await provider.refreshHistory()
        }
        return result
      }
      return action === 'apply' ? provider.commitCommand(commit, false) : commit()
    },
    async close(id) {
      return transport.invoke('comparison_close', { id })
    },
    async preview(id, side) {
      return transport.invoke('comparison_preview', { id, side })
    },
    async listen(changed) {
      const dispose = await Promise.all(
        ['web-editor:change', 'comparison:changed', 'agent:changed'].map((event) =>
          transport.listen(event, changed),
        ),
      )
      return () => dispose.forEach((fn) => fn())
    },
  }
}

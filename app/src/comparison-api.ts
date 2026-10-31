import * as Y from 'yjs'
import { CONFIRMED_COMMAND, type Provider, type Transport, type Resource } from './provider'
import type { ComparisonAPI } from './comparison-types'
import { tr } from './i18n'
export function comparisonAPI(provider: Provider, transport: Transport): ComparisonAPI {
  const generation = provider.generation
  function checkCurrent() {
    if (provider.composing) throw new Error(tr('Terminez la saisie en cours avant de décider.'))
    if (provider.generation !== generation) throw new Error(tr('Le document a changé. Réessayez.'))
  }
  async function ready() {
    checkCurrent()
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
    async apply(id, choices) {
      await ready()
      return provider.commitCommand(async () => {
        // Do not flush from inside the queued command: its tail includes itself.
        checkCurrent()
        const current = await transport.invoke<Awaited<ReturnType<ComparisonAPI['read']>>>(
          'comparison_read',
          { id },
        )
        const destination = current.report.path
          ? undefined
          : await transport.invoke<string | null>('choose_file', { save: true })
        if (destination === null) return null
        checkCurrent()
        const result = await transport.invoke<
          NonNullable<Awaited<ReturnType<ComparisonAPI['apply']>>> & {
            publication?: {
              guard: { document: string; revision: number }
              update: number[]
              resources: Resource[]
            }
          }
        >('comparison_apply', { id, choices, destination })
        if (provider.generation !== generation || provider.rejected) return result
        if (result.publication) {
          for (const resource of result.publication.resources) provider.addResource(resource)
          Y.applyUpdate(provider.doc, new Uint8Array(result.publication.update), CONFIRMED_COMMAND)
          provider.revision = Math.max(provider.revision, result.publication.guard.revision)
          await provider.refreshHistory().catch((error) => provider.onError(error))
        }
        provider.confirmSaved(result.saved)
        return result
      })
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

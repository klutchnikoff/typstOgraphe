import * as Y from 'yjs'
import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import { stableJSON } from './provider'
import { settingsFor } from './state'
import { documentContent } from './shared-inline-marks'
import { privateDrafts, DRAFT_CHANGED } from './private-drafts'
import { RecoveryScheduler } from './recovery-scheduler'
import { tr } from './i18n'

/** Snapshot private input without flushing it into the document. The backend owns the confirmed base. */
export function privateRecovery(
  editor: Editor,
  provider: Provider,
  active: () => boolean,
  failed: (error: unknown) => void,
) {
  const generation = provider.generation
  const forms = privateDrafts(provider)
  let sequence = 0,
    last = '',
    frozen: ReturnType<typeof capture> | undefined,
    composingTarget: Node | undefined
  const epoch = provider.transport.invoke<number>('open_private_recovery', { generation })
  // Attach the rejection handler immediately, even if the welcome screen stays idle.
  void epoch.catch(failed)
  function capture() {
    return {
      update: [...Y.encodeStateAsUpdate(provider.doc)],
      document: documentContent(editor.getJSON()),
      settings: settingsFor(provider),
      fields: provider.documentEdits?.draft?.() || [],
      forms: forms.snapshot(),
      resources: [...provider.resources.values()],
      rejected: !!provider.rejected,
    }
  }
  const scheduler = new RecoveryScheduler(
    async () => {
      if (!active() || provider.generation !== generation) return
      const version = await epoch
      if (!active() || provider.generation !== generation) return
      if (composingTarget && !composingTarget.isConnected) {
        frozen = undefined
        composingTarget = undefined
      }
      const input = frozen || capture(),
        key = stableJSON(input)
      if (key === last) return
      const saved = await provider.transport.invoke<boolean>('conserve_private', {
        input: { generation, epoch: version, sequence: ++sequence, ...input },
      })
      if (!saved) throw new Error('recovery_superseded_snapshot')
      last = key
    },
    (error) => {
      if (provider.generation === generation)
        failed(
          `${tr('La copie de secours n’a pas pu être enregistrée. Votre travail reste dans cette fenêtre.')} ${String(error)}`,
        )
    },
  )
  const changed = () => {
    if (active()) scheduler.changed()
  }
  const fieldChanged = () => changed()
  const formChanged = (event: Event) => {
    if ((event as CustomEvent).detail === provider) changed()
  }
  const begin = (event: Event) => {
    frozen ??= capture()
    composingTarget = event.target instanceof Node ? event.target : undefined
  }
  const end = () =>
    queueMicrotask(() => {
      frozen = undefined
      composingTarget = undefined
      changed()
    })
  provider.doc.on('update', changed)
  document.addEventListener('metadata:draft', fieldChanged)
  document.addEventListener(DRAFT_CHANGED, formChanged)
  document.addEventListener('compositionstart', begin, true)
  document.addEventListener('compositionend', end)
  const release = () => {
    scheduler.stop()
    provider.doc.off('update', changed)
    document.removeEventListener('metadata:draft', fieldChanged)
    document.removeEventListener(DRAFT_CHANGED, formChanged)
    document.removeEventListener('compositionstart', begin, true)
    document.removeEventListener('compositionend', end)
  }
  editor.on('destroy', release)
  const result = {
    changed,
    flush: () =>
      scheduler.flush().catch((error) => {
        throw new Error(
          `${tr('La copie de secours n’a pas pu être enregistrée. Votre travail reste dans cette fenêtre.')} ${String(error)}`,
        )
      }),
    stop: release,
  }
  provider.privateRecovery = result
  return result
}

import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import { documentContent } from './shared-inline-marks'
import { settingsFor } from './state'
import type { Provider, Transport, Bootstrap } from './provider'

/** Explicit native recipe only: the normal form, scheduler, save and reopen paths. */
export async function draftDiagnostic(
  mode: string,
  transport: Transport,
  provider: Provider,
  editor: Editor,
) {
  const marker = 'Native unconfirmed note é😀'
  try {
    if (mode === 'draft-verify') {
      const joined = await transport.invoke<Bootstrap>('join')
      if (!joined.opened) {
        try {
          await transport.invoke('open_file', {
            path: `${provider.sessionPath}/project.typsto`,
            revision: provider.revision,
          })
        } catch (error) {
          const current = await transport.invoke<Bootstrap>('join')
          if (current.generation !== joined.generation && current.opened) return
          throw error
        }
        return
      }
      const records = await transport.invoke<unknown[]>('diagnostic_private_records')
      if (!records.some((r) => JSON.stringify(r).includes(marker)))
        throw new Error('Private input lost after process interruption')
      if (JSON.stringify(editor.getJSON()).includes(marker))
        throw new Error('Private note was published during reopen')
      await transport.invoke('diagnostic_finish', {
        report: {
          success: true,
          checks: [
            'private input survives process interruption',
            'confirmed archive reopens without private publication',
            'confirmed recovery checkpoint validates',
          ],
        },
      })
      return
    }
    await provider.flush()
    document.querySelector<HTMLElement>('.footnote-ref .inline-object-label')!.click()
    const input = document.querySelector<HTMLTextAreaElement>('[aria-label="Texte de la note"]')!
    input.focus()
    input.value = marker
    input.dispatchEvent(new Event('input', { bubbles: true }))
    // Exercise automatic scheduling, not an explicit test flush.
    const deadline = Date.now() + 20000
    for (;;) {
      await new Promise((resolve) => setTimeout(resolve, 200))
      const records = await transport.invoke<unknown[]>('diagnostic_private_records')
      if (records.some((r) => JSON.stringify(r).includes(marker))) break
      if (Date.now() > deadline) throw new Error('Automatic private checkpoint absent')
    }
    await transport.invoke('save_file', {
      path: `${provider.sessionPath}/project.typsto`,
      snapshot: {
        revision: provider.revision,
        document: documentContent(editor.getJSON()),
        settings: settingsFor(provider),
      },
      update: [...Y.encodeStateAsUpdate(provider.doc)],
    })
    const after = await transport.invoke<unknown[]>('diagnostic_private_records')
    if (!after.some((r) => JSON.stringify(r).includes(marker)))
      throw new Error('Saving confirmed state removed private form')
    await transport.invoke('diagnostic_event', {
      target: 'main',
      step: 'progress',
      payload: { draftConservationReady: true },
    })
    // The runner now kills this process, bypassing all normal close cleanup.
  } catch (error) {
    await transport.invoke('diagnostic_finish', {
      report: { success: false, error: String(error) },
    })
  }
}

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
    if (mode === 'draft-recover') {
      const wait = async (test: () => unknown) => {
        const deadline = Date.now() + 20000
        while (!test()) {
          if (Date.now() > deadline) throw new Error('Recovery interface timed out')
          await new Promise((resolve) => setTimeout(resolve, 40))
        }
      }
      if (!provider.recoveredName) {
        await wait(() =>
          document.querySelector<HTMLButtonElement>('.recovery-work-list button.primary'),
        )
        document.querySelector<HTMLButtonElement>('.recovery-work-list button.primary')!.click()
        return // The ordinary document transition reloads this webview.
      }
      if ((await transport.invoke<Bootstrap>('join')).filePath)
        throw new Error('Recovered document still owns the original file')
      if (JSON.stringify(editor.getJSON()).includes(marker))
        throw new Error('Private form was published during recovery')
      const resume = () => document.querySelector<HTMLButtonElement>('[data-recovered-id]')!.click()
      await wait(() => document.querySelector('[data-recovered-id]'))
      await transport.invoke('work_export', {
        id: provider.recoveredItems[0].id,
        path: `${provider.sessionPath}/exported-draft.json`,
      })
      resume()
      const input = () =>
        document.querySelector<HTMLTextAreaElement>('[aria-label="Texte de la note"]')!
      await wait(() => input()?.value === marker)
      input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
      if (provider.recoveredItems.length !== 1) throw new Error('Cancel consumed recovered form')
      resume()
      await wait(() => input()?.value === marker)
      input().dispatchEvent(
        new KeyboardEvent('keydown', { key: 'Enter', metaKey: true, bubbles: true }),
      )
      await wait(() => !provider.recoveredItems.length)
      if (!JSON.stringify(editor.getJSON()).includes(marker))
        throw new Error('Validated note absent')
      await provider.privateRecovery!.force()
      const records = await transport.invoke<any[]>('diagnostic_private_records')
      if (
        !records.some(
          (r) =>
            !r.input.recovered.length &&
            !r.input.forms.length &&
            JSON.stringify(r.confirmed.document).includes(marker),
        )
      )
        throw new Error('Validated recovery was not durable')
      await transport.invoke('diagnostic_finish', {
        report: {
          success: true,
          checks: [
            'welcome lists interrupted work',
            'resume creates independent project without original lease',
            'ordinary band restores private note without publication',
            'Escape retains recovered source',
            'Cmd Enter publishes through typed command',
            'consumption follows durable acknowledgement',
          ],
        },
      })
      return
    }
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

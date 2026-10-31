import { withCurrentSnapshot } from './snapshot-operation'
import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import { documentContent } from './shared-inline-marks'
import { settingsFor } from './state'
import { stableJSON, type Bootstrap, type Provider, type Transport } from './provider'

const marker = 'Private publication note é😀 — にほん'
async function until(check: () => boolean | Promise<boolean>, label: string) {
  const deadline = Date.now() + 25000
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(`Publication timeout: ${label}`)
    await new Promise((resolve) => setTimeout(resolve, 40))
  }
}
/** Explicit native acceptance only. Uses the ordinary IPC, editor and form collector. */
export async function publicationDiagnostic(
  mode: string,
  transport: Transport,
  provider: Provider,
  editor: Editor,
) {
  let obsolete = false
  window.addEventListener(
    'beforeunload',
    () => {
      obsolete = true
    },
    { once: true },
  )
  const checks: string[] = []
  const state = () => ({
    document: documentContent(editor.getJSON()),
    settings: settingsFor(provider),
  })
  const snapshot = () => ({ revision: provider.revision, ...state() })
  const records = () => transport.invoke<any[]>('diagnostic_private_records')
  const save = (file: string) =>
    withCurrentSnapshot(
      provider,
      () => ({ snapshot: snapshot(), update: [...Y.encodeStateAsUpdate(provider.doc)] }),
      (checkpoint) =>
        transport.invoke('save_file', { path: `${provider.sessionPath}/${file}`, ...checkpoint }),
      false,
    )
  const published = async () => {
    const final = (await transport.invoke<{ state: unknown }>('diagnostic_confirmed')).state
    const exported = await transport.invoke<string>('export_document', { snapshot: snapshot() })
    const preview = await transport.invoke<{ pageSizes: number[][] }>('preview', {
      snapshot: snapshot(),
    })
    return { final, exported, pageSizes: preview.pageSizes }
  }
  const fingerprint = async () =>
    stableJSON({
      ...state(),
      revision: provider.revision,
      update: [...Y.encodeStateAsUpdate(provider.doc)],
      resources: [...provider.resources],
      forms: (await records())
        .filter((r) => r.input.generation === provider.generation)
        .map((r) => r.input.forms),
      file: (await transport.invoke<Bootstrap>('join')).filePath,
    })
  const refuse = async (operation: () => Promise<unknown>, label: string) => {
    const before = await fingerprint()
    let failed = false,
      reason = ''
    try {
      await operation()
    } catch (error) {
      failed = true
      reason = String(error)
    }
    if (!failed) throw new Error(`Unexpected successful ${label}`)
    if (label === 'external modification refusal' && !reason.includes('modifié en dehors'))
      throw new Error(`Wrong refusal: ${reason}`)
    if ((await fingerprint()) !== before) throw new Error(`${label} changed current work`)
    checks.push(`${label}: state, revision, CRDT, private form, resources and lease preserved`)
  }
  try {
    await provider.flush(false)
    const joined = await transport.invoke<Bootstrap>('join')
    if (!joined.opened) {
      try {
        await transport.invoke('open_file', {
          path: `${provider.sessionPath}/${mode.endsWith('verify') ? 'independent.typsto' : 'project.typsto'}`,
          revision: provider.revision,
        })
      } catch (error) {
        // A WebKit invoke retried during reload cannot fail the replacement view.
        const current = await transport.invoke<Bootstrap>('join')
        if (current.generation !== joined.generation && current.opened) return
        throw error
      }
      return // Ordinary document replacement reloads this webview.
    }
    await provider.flush()
    const kind = state().settings.metadata.type
    checks.push('native project and confirmed editing state are ready')
    if (mode.endsWith('verify')) {
      if (stableJSON(state()).includes(marker))
        throw new Error('Private form published during reopen')
      if (!(await records()).some((r) => JSON.stringify(r.input.forms).includes(marker)))
        throw new Error('Private form lost after process interruption')
      checks.push('confirmed copy reopens after interruption without publishing the private note')
      await transport.invoke('diagnostic_finish', {
        report: { success: true, kind, checks, ...(await published()) },
      })
      return
    }
    editor.commands.setTextSelection(editor.state.doc.content.size - 1)
    editor.commands.insertContent(' Publication A.3 — contenu confirmé é😀.')
    await provider.flush()
    document.querySelector<HTMLButtonElement>('[data-command="insert.footnote"]')!.click()
    await until(
      () => !!document.querySelector('[aria-label="Texte de la note"]'),
      'note band opens',
    )
    const note = document.querySelector<HTMLTextAreaElement>('[aria-label="Texte de la note"]')!
    note.value = marker
    note.dispatchEvent(new Event('input', { bubbles: true }))
    await provider.privateRecovery!.force()
    await save('project.typsto')
    if (!(await records()).some((r) => JSON.stringify(r.input.forms).includes(marker)))
      throw new Error('Confirmed save removed private form')
    checks.push('explicit archive save retains unconfirmed form')
    await transport.invoke('diagnostic_event', {
      target: 'main',
      step: 'progress',
      payload: { publicationExternalReady: true },
    })
    await until(
      () => transport.invoke<boolean>('diagnostic_external_ready'),
      'external process mutation',
    )
    await refuse(() => save('project.typsto'), 'external modification refusal')
    await refuse(() => save('blocked-parent/failed.typsto'), 'storage failure')
    await refuse(
      () =>
        transport.invoke('open_file', {
          path: `${provider.sessionPath}/corrupt.typsto`,
          revision: provider.revision,
        }),
      'incoherent archive refusal',
    )
    await save('independent.typsto')
    if (!(await records()).some((r) => JSON.stringify(r.input.forms).includes(marker)))
      throw new Error('Save as removed private form')
    checks.push('save as succeeds after refusal and retains the private note')
    await transport.invoke('diagnostic_event', {
      target: 'main',
      step: 'progress',
      payload: {
        publicationReady: true,
        success: true,
        kind,
        checks,
        ...(await published()),
      },
    })
    // The runner kills only this process after checking the external file bytes.
  } catch (error) {
    if (!obsolete)
      await transport.invoke('diagnostic_finish', {
        report: { success: false, checks, error: String(error) },
      })
  }
}

import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import { documentContent } from './shared-inline-marks'
import { settingsFor } from './state'
import { stableJSON, type Bootstrap, type Provider, type Transport } from './provider'
import { until, clickControl, comparisonDialog, finishComparison } from './closure-controls'
function append(editor: Editor, text: string) {
  editor.commands.setTextSelection(editor.state.doc.content.size - 1)
  editor.view.focus()
  editor.commands.insertContent(text)
}
type Proposal = { id: string; requestId: string; status: string }
/** Real native UI and IPC; loaded only for the explicit closure recipe. */
export async function closureDiagnostic(
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
  const steps: string[] = []
  const controls = () => ({
    revision: provider.revision,
    editable: editor.isEditable,
    historyBusy: provider.historyBusy,
    online: provider.online,
    history: provider.historyState,
    focused: document.activeElement?.outerHTML.slice(0, 300),
    dialogs: [...document.querySelectorAll('dialog')].map((dialog) => ({
      open: dialog.open,
      text: dialog.textContent?.slice(-1500),
      buttons: [...dialog.querySelectorAll('button')].map((button) => ({
        action: button.dataset.action,
        text: button.textContent,
        disabled: button.disabled,
        inert: !!button.closest('[inert]'),
      })),
    })),
  })
  const progress = (phase: string) =>
    transport.invoke('diagnostic_event', {
      target: provider.id,
      step: 'progress',
      payload: { phase, revision: provider.revision },
    })
  const state = () => ({
    document: documentContent(editor.getJSON()),
    settings: settingsFor(provider),
  })
  const snapshot = () => ({ revision: provider.revision, ...state() })
  const proposal = async () =>
    (await transport.invoke<{ proposal: Proposal | null }>('agent_state')).proposal
  const pending = async (id: string) => {
    await until(async () => {
      const p = await proposal()
      return p?.requestId === id && p.status === 'pending'
    }, id)
    return (await proposal())!
  }
  const save = () =>
    transport.invoke('save_file', {
      path: `${provider.sessionPath}/project.typsto`,
      snapshot: snapshot(),
      update: [...Y.encodeStateAsUpdate(provider.doc)],
    })
  const openAssistance = async () => {
    document.querySelector<HTMLButtonElement>('[aria-label="Assistance à la rédaction"]')!.click()
    await until(() => !!document.querySelector('.side-panel h3'), 'assistance panel')
  }
  const button = (text: string) =>
    [...document.querySelectorAll('button')].find((b) => b.textContent === text)
  try {
    const joined = await transport.invoke<Bootstrap>('join')
    if (!joined.opened) {
      await provider.flush()
      await provider.setSynchronization(true)
      await provider.flush()
      try {
        await transport.invoke('open_file', {
          path: `${provider.sessionPath}/project.typsto`,
          revision: provider.revision,
        })
      } catch (error) {
        // WebKit can retry an invoke whose reply was interrupted by the ordinary
        // app:document reload. A successful first open changed the generation;
        // the obsolete page must leave the result to its replacement.
        const current = await transport.invoke<Bootstrap>('join')
        if (current.generation !== joined.generation && current.opened) return
        throw error
      }
      return // The ordinary document lifecycle reloads the real editor.
    }
    await provider.flush()
    if (provider.synchronization?.enabled)
      throw new Error('New document inherited presence services')
    const initial = (
      await transport.invoke<{ state: ReturnType<typeof state> }>('diagnostic_confirmed')
    ).state
    const anchor = () =>
      JSON.parse(
        JSON.stringify(
          Y.createRelativePositionFromTypeIndex(provider.doc.getXmlFragment('default'), 0),
        ),
      )
    const initialAnchor = anchor()
    const kind = initial.settings.metadata.type
    if (!['courrier', 'article'].includes(kind)) throw new Error('Unexpected document type')
    steps.push('Native open: project, embedded model, metadata and resources validated.')
    if (mode === 'closure') {
      if (kind === 'courrier') {
        document.getElementById('metadata-tab-header')!.click()
        const input = document.getElementById('metadata-field-letter.subject') as HTMLInputElement
        if (!input) throw new Error('Letter subject control absent')
        input.focus()
        input.value += ' — relu dans Tauri'
        input.dispatchEvent(new Event('input', { bubbles: true }))
      } else {
        document.getElementById('metadata-tab-abstracts')!.click()
        await until(
          () =>
            !!document.querySelector(
              '[data-section="localizations"] [data-rich-zone-profile="rich-text"]',
            ),
          'abstract field',
        )
        const element = document.querySelector(
          '[data-section="localizations"] [data-rich-zone-profile="rich-text"]',
        ) as HTMLElement & { editor: Editor }
        append(element.editor, ' [TAURI-RÉSUMÉ]')
      }
      await provider.flush()
      append(editor, '[TAURI]')
      await provider.flush()
      const beforeUndo = stableJSON(state())
      editor.commands.undo()
      await until(() => !editor.getText().includes('[TAURI]'), 'human undo')
      editor.commands.redo()
      await until(() => stableJSON(state()) === beforeUndo, 'human redo')
      steps.push('Native header/body input, undo and redo preserve the project.')
      await save()
      await transport.invoke('agent_invite')
      await pending('closure-refused')
      const beforeRefusal = stableJSON(state())
      await openAssistance()
      await until(() => !!button('Refuser le lot'), 'reject button')
      button('Refuser le lot')!.click()
      await until(async () => (await proposal())?.status === 'rejected', 'human rejection')
      if (stableJSON(state()) !== beforeRefusal) throw new Error('Rejection modified the project')
      steps.push('Real attached MCP: rejected proposal never published.')
      const stale = await pending('closure-stale')
      append(editor, ' [TAURI-CONCURRENT]')
      await provider.flush()
      const beforeStale = stableJSON(state())
      try {
        await transport.invoke('agent_accept', { id: stale.id })
        throw new Error('Stale lot accepted')
      } catch (error) {
        if (!String(error).includes('périmée')) throw error
      }
      if (stableJSON(state()) !== beforeStale) throw new Error('Stale proposal changed the project')
      steps.push('Concurrent human input invalidates the guarded MCP proposal without losing work.')
      await pending('closure-accepted')
      const before = stableJSON(state())
      editor.commands.setTextSelection(2)
      editor.view.focus()
      await provider.flush()
      await provider.refreshHistory()
      const selection = editor.state.selection.toJSON(),
        undo = provider.historyState.undo_steps
      await openAssistance()
      await until(() => !!button('Examiner la proposition'), 'review button')
      button('Examiner la proposition')!.click()
      await until(
        () => !!document.querySelector('[data-action="validate-comparison"]'),
        'comparison',
      )
      const review = comparisonDialog()
      const validate = review.querySelector<HTMLButtonElement>(
        '[data-action="validate-comparison"]',
      )!
      if (validate.disabled || !validate.textContent?.startsWith('Appliquer '))
        throw new Error('Missing explicit change count')
      await clickControl(review, '[data-action="validate-comparison"]', 'proposal application')
      await until(() => editor.getText().includes('[MCP-ACCEPTED]'), 'published lot')
      await finishComparison(review)
      await provider.flush(false)
      await provider.refreshHistory()
      if (stableJSON(editor.state.selection.toJSON()) !== stableJSON(selection))
        throw new Error('Human selection moved')
      if (provider.historyState.undo_steps !== undo + 1)
        throw new Error('MCP lot is not one undo step')
      const accepted = stableJSON(state())
      editor.view.focus()
      if (!editor.commands.undo()) throw new Error('Whole-lot undo is unavailable')
      await until(() => stableJSON(state()) === before, 'single whole-lot undo')
      if (!editor.commands.redo()) throw new Error('Whole-lot redo is unavailable')
      await until(() => stableJSON(state()) === accepted, 'whole-lot redo')
      if (provider.synchronization?.enabled) throw new Error('Agent enabled human presence')
      steps.push(
        'Real human approval, stable selection, one whole-lot undo/redo; individual mode preserved.',
      )
      await provider.flush()
      await save()
      const { versionsAPI } = await import('./versions-api')
      const { versionDialog } = await import('./version-dialog')
      const { comparisonAPI } = await import('./comparison-api')
      const api = versionsAPI({
        provider,
        transport,
        destination: async () => undefined,
        saved: async () => {},
      })
      await progress('versions.begin')
      const index = await api.list()
      const created = await api.change(index.history, {
        kind: 'create',
        details: { name: 'Closure reference', tags: ['recipe'], description: null, creator: null },
      })
      if (!created) throw new Error('Version not created')
      await progress('versions.created')
      const record = created.versions.find((item) => item.details.name === 'Closure reference')!
      const reference = stableJSON(state())
      append(editor, ' [AFTER-VERSION]')
      await provider.flush()
      const modified = stableJSON(state())
      await progress('versions.edited')
      const catalogue = await transport.invoke<import('./shell').Catalogue>('content_catalogue')
      versionDialog({
        record,
        index: created,
        api,
        catalogue,
        returnFocus: () => undefined,
        changed: () => {},
      })
      const dialog = comparisonDialog()
      await until(() => !!dialog.querySelector('.version-prose'), 'version content')
      await clickControl(dialog, '[data-view="compare"]', 'version comparison tab')
      await clickControl(dialog, '[data-action="select-all-before"]', 'version comparison')
      await clickControl(dialog, '[data-action="validate-comparison"]', 'version application')
      await until(() => stableJSON(state()) === reference, 'version reprise')
      await progress('versions.content-installed')
      await finishComparison(dialog)
      await progress('versions.closed')
      editor.view.focus()
      if (!editor.commands.undo()) throw new Error('Version undo is unavailable')
      await until(() => stableJSON(state()) === modified, 'version undo')
      await progress('versions.undone')
      if (!editor.commands.redo()) throw new Error('Version redo is unavailable')
      await until(() => stableJSON(state()) === reference, 'version redo')
      await progress('versions.redone')
      const frozen = await comparisonAPI(provider, transport).open({
        kind: 'version',
        id: record.id,
      })
      if (frozen.report.groups.length) throw new Error('Reprise differs from reference')
      await transport.invoke('comparison_close', { id: frozen.id })
      await save()
      steps.push('Durable version: read-only modal, reprise, single undo/redo and explicit save.')
    } else {
      for (const marker of ['[TAURI]', '[CLI]', '[MCP-AUTONOMOUS]', '[MCP-ACCEPTED]'])
        if (!editor.getText().includes(marker)) throw new Error(`Missing ${marker}`)
      if (kind === 'article' && !JSON.stringify(initial.settings).includes('[TAURI-RÉSUMÉ]'))
        throw new Error('Rich header lost')
      steps.push(
        'Final native reopen preserves Tauri, CLI and autonomous/attached MCP contributions.',
      )
      await save()
    }
    await progress('export.begin')
    const exported = await transport.invoke<string>('export_document', { snapshot: snapshot() })
    const preview = await transport.invoke<{ pageSizes: number[][] }>('preview', {
      snapshot: snapshot(),
    })
    await transport.invoke('diagnostic_finish', {
      report: {
        success: true,
        mode,
        kind,
        steps,
        initial,
        initialAnchor,
        finalAnchor: anchor(),
        final: (await transport.invoke<{ state: ReturnType<typeof state> }>('diagnostic_confirmed'))
          .state,
        resources: [...provider.resources.keys()].sort(),
        exported,
        pageSizes: preview.pageSizes,
      },
    })
  } catch (error) {
    // Opening a file uses the ordinary app:document reload. WebKit may reject
    // an old invoke while that page unloads; it cannot decide the new view's result.
    if (obsolete) return
    await transport.invoke('diagnostic_finish', {
      report: {
        success: false,
        mode,
        steps,
        error: String(error),
        state: state(),
        controls: controls(),
      },
    })
  }
}

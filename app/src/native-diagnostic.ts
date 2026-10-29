import { historyFor } from './editing-history'
import { documentContent } from './shared-inline-marks'
import type { Editor, JSONContent } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import * as Y from 'yjs'
import { settingsFor } from './state'

const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
async function until(check: () => boolean | Promise<boolean>, message: string) {
  const deadline = Date.now() + 30_000
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(message)
    await pause(60)
  }
}
function append(editor: Editor, text: string) {
  historyFor(editor)?.stopCapturing()
  editor.commands.insertContentAt(editor.state.doc.content.size, {
    type: 'paragraph',
    content: [{ type: 'text', text }],
  })
}

/** Recette des fenêtres Tauri, activée uniquement par un argument CLI. */
export async function nativeDiagnostic(
  mode: string,
  transport: Transport,
  provider: Provider,
  editor: Editor,
  save: () => Promise<void>,
) {
  type Command = { id: number; action: string; text?: string }
  type Result = { text: string; document: JSONContent }
  type Event = {
    from: string
    step: string
    payload: Command & Result & { value: Result; error?: string }
  }
  let ready = ''
  let counter = 0
  const pending = new Map<
    number,
    { resolve: (value: Result) => void; reject: (error: unknown) => void }
  >()
  const send = (target: string, step: string, payload: unknown) =>
    transport.invoke('diagnostic_event', { target, step, payload })
  const perform = async (command: Command): Promise<Result> => {
    if (command.action === 'append') append(editor, command.text!)
    if (command.action === 'disconnect') await provider.disconnect()
    if (command.action === 'reconnect') await provider.reconnect()
    if (command.action === 'undo') editor.commands.undo()
    if (command.action === 'redo') editor.commands.redo()
    if (command.action === 'annotations') {
      const targets: { pos: number; kind: string }[] = []
      editor.state.doc.descendants((node, pos) => {
        if (['footnote', 'figure'].includes(node.type.name))
          targets.push({ pos, kind: node.type.name })
      })
      for (const target of targets.reverse()) {
        const node = editor.state.doc.nodeAt(target.pos)!
        if (target.kind === 'footnote')
          editor.view.dispatch(
            editor.state.tr.setNodeMarkup(target.pos, undefined, {
              ...node.attrs,
              text: 'Note relue à deux : Noël et にほん.',
            }),
          )
        else editor.commands.insertContentAt(target.pos + 1, 'Légende native : ')
      }
    }
    if (provider.online) await provider.flush()
    return { text: editor.getText(), document: documentContent(editor.getJSON()) }
  }
  await transport.listen<Event>('web-editor:diagnostic', (event) => {
    if (event.step === 'ready') ready = event.from
    if (event.step === 'result') {
      const request = pending.get(event.payload.id)
      pending.delete(event.payload.id)
      if (event.payload.error) request?.reject(event.payload.error)
      else request?.resolve(event.payload.value)
    }
    if (event.step === 'execute' && provider.id !== 'main') {
      void perform(event.payload)
        .then((value) => send('main', 'result', { id: event.payload.id, value }))
        .catch((error) => send('main', 'result', { id: event.payload.id, error: String(error) }))
    }
  })
  if (provider.id !== 'main') {
    await send('main', 'ready', {})
    return
  }
  const steps: string[] = []
  const command = async (action: string, text?: string) => {
    const id = ++counter
    const promise = new Promise<Result>((resolve, reject) => {
      pending.set(id, { resolve, reject })
      setTimeout(() => {
        if (pending.delete(id)) reject(new Error(`Participant muet : ${action}`))
      }, 30_000)
    })
    await send(ready, 'execute', { id, action, text })
    return promise
  }
  try {
    if (mode === 'restore') {
      if (
        !editor.getText().includes('NATIVE-CAMILLE') ||
        !JSON.stringify(documentContent(editor.getJSON())).includes('Note relue à deux')
      )
        throw new Error('Reprise native incomplète.')
      steps.push('Réouverture native : Typst, état Yjs, note et ressources cohérents.')
      await save()
      steps.push('Nouvel enregistrement après réouverture.')
    } else {
      await transport.invoke('open_peer')
      await until(() => Boolean(ready), 'Deuxième fenêtre non initialisée.')
      steps.push('Deux fenêtres Tauri, deux Y.Doc et un relais Rust opérationnels.')
      await command('disconnect')
      await command('append', 'NATIVE-ALEX')
      append(editor, 'NATIVE-CAMILLE')
      await provider.flush()
      await command('reconnect')
      await until(() => editor.getText().includes('NATIVE-ALEX'), 'Fusion hors ligne incomplète.')
      steps.push('Éditions concurrentes puis convergence après reconnexion.')
      await command('undo')
      await until(() => !editor.getText().includes('NATIVE-ALEX'), 'Annulation distante non reçue.')
      if (!editor.getText().includes('NATIVE-CAMILLE'))
        throw new Error('Annulation du travail du voisin.')
      await command('redo')
      await until(() => editor.getText().includes('NATIVE-ALEX'), 'Rétablissement non reçu.')
      steps.push('Annulation locale et rétablissement sans supprimer le travail du voisin.')
      await command('annotations')
      await until(
        () => JSON.stringify(documentContent(editor.getJSON())).includes('Note relue à deux'),
        'Annotation distante absente.',
      )
      const paragraph = await transport.invoke<JSONContent>('prepare_rust_edit')
      editor.commands.insertContentAt(editor.state.doc.content.size, paragraph)
      await provider.flush()
      await until(
        async () =>
          (await command('inspect')).text.includes('Cette phrase a été préparée depuis Rust'),
        'Commande Rust non partagée.',
      )
      steps.push('Notes/légendes et commande préparée par Rust partagées par le même circuit.')
      await pause(300)
      await provider.flush()
      await transport.invoke('agent_invite')
      await until(async () => {
        const state = await transport.invoke<{ proposal?: { status: string } }>('agent_state')
        return state.proposal?.status === 'pending'
      }, 'Proposition MCP absente.')
      if (editor.getText().includes('NATIVE-MCP'))
        throw new Error('Proposition publiée avant décision.')
      const caret = editor.state.selection.from
      document.querySelector<HTMLButtonElement>('[aria-label="Assistance à la rédaction"]')!.click()
      await until(
        () =>
          Array.from(document.querySelectorAll('button')).some(
            (b) => b.textContent === 'Accepter le lot',
          ),
        'Relecture absente.',
      )
      Array.from(document.querySelectorAll('button'))
        .find((b) => b.textContent === 'Accepter le lot')!
        .click()
      await until(() => editor.getText().includes('NATIVE-MCP'), 'Acceptation absente.')
      if (editor.state.selection.from !== caret) throw new Error('Curseur humain déplacé.')
      await until(
        async () => (await command('inspect')).text.includes('NATIVE-MCP'),
        'Lot non partagé.',
      )
      editor.commands.undo()
      await until(
        async () => !(await command('inspect')).text.includes('NATIVE-MCP'),
        'Lot non annulé en une fois.',
      )
      steps.push(
        'MCP STDIO réel : découverte, proposition non publiée, relecture et acceptation du lot, sélection préservée et annulation partagée.',
      )
      const bootstrap = await transport.invoke<{ sessionPath: string }>('join')
      await provider.flush()
      await transport.invoke('save_file', {
        path: `${bootstrap.sessionPath}/courrier.typsto`,
        snapshot: {
          revision: provider.revision,
          document: documentContent(editor.getJSON()),
          settings: settingsFor(provider),
        },
        update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
      })
      steps.push('Courrier enregistré dans une archive .typsto autonome sous verrou.')
      await send('main', 'progress', { phase: 'before checkpoint' })
      await save()
      await send('main', 'progress', { phase: 'before export' })
      const path = await transport.invoke<string>('export_document', {
        snapshot: { revision: provider.revision, document: documentContent(editor.getJSON()) },
      })
      await send('main', 'progress', { phase: 'export complete' })
      steps.push(`Sauvegarde atomique et export PDF/Typst réel : ${path}`)
    }
    await transport.invoke('diagnostic_finish', { report: { success: true, mode, steps } })
  } catch (error) {
    await transport.invoke('diagnostic_finish', {
      report: { success: false, mode, steps, error: String(error) },
    })
  }
}

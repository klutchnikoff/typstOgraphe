import type { Editor, JSONContent } from '@tiptap/core'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import type { Provider, Transport } from './provider'

const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
async function until(check: () => boolean | Promise<boolean>, message: string) {
  const deadline = Date.now() + 30_000
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(message)
    await pause(60)
  }
}
function append(editor: Editor, text: string) {
  yUndoPluginKey.getState(editor.state)?.undoManager.stopCapturing()
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
    return { text: editor.getText(), document: editor.getJSON() }
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
        !JSON.stringify(editor.getJSON()).includes('Note relue à deux')
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
        () => JSON.stringify(editor.getJSON()).includes('Note relue à deux'),
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
      await save()
      const path = await transport.invoke<string>('export_document', {
        snapshot: { revision: provider.revision, document: editor.getJSON() },
      })
      steps.push(`Sauvegarde atomique et export PDF/Typst réel : ${path}`)
    }
    await transport.invoke('diagnostic_finish', { report: { success: true, mode, steps } })
  } catch (error) {
    await transport.invoke('diagnostic_finish', {
      report: { success: false, mode, steps, error: String(error) },
    })
  }
}

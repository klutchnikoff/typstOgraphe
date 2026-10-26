import { tr } from './i18n'
import { icon } from './icons'
import { Editor, type JSONContent } from '@tiptap/core'
import Collaboration from '@tiptap/extension-collaboration'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { baseExtensions } from './extensions'
import type { Provider, Transport } from './provider'
interface Proposal {
  id: string
  summary: string
  revision: number
  status: string
  before: JSONContent
  after: JSONContent
  stages: JSONContent[]
}
export function prepareProposal(provider: Provider, stages: JSONContent[]): Uint8Array {
  const draft = new Y.Doc()
  Y.applyUpdate(draft, Y.encodeStateAsUpdate(provider.doc))
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: [
      ...baseExtensions((path) => provider.imageURL(path)),
      Collaboration.configure({ document: draft, field: 'default' }),
    ],
  })
  try {
    for (const stage of stages) {
      const next = editor.schema.nodeFromJSON(stage)
      next.check()
      const before = editor.state.doc
      const start = before.content.findDiffStart(next.content)
      if (start === null) continue
      const end = before.content.findDiffEnd(next.content)
      if (!end) continue
      const overlap = start - Math.min(end.a, end.b)
      const oldEnd = end.a + Math.max(0, overlap),
        newEnd = end.b + Math.max(0, overlap)
      editor.view.dispatch(editor.state.tr.replace(start, oldEnd, next.slice(start, newEnd)))
    }
    return Y.encodeStateAsUpdate(draft, Y.encodeStateVector(provider.doc))
  } finally {
    editor.destroy()
    draft.destroy()
  }
}
export function mountAssistance(
  provider: Provider,
  transport: Transport,
  editor: Editor,
  showPanel: (title: string) => HTMLElement,
  message: (text: string, error?: boolean) => void,
  isTauri: boolean,
) {
  const button = document.createElement('button')
  button.innerHTML = icon('pen-line')
  button.title = tr('Assistance à la rédaction')
  button.setAttribute('aria-label', tr('Assistance à la rédaction'))
  document.querySelector('.side-rail')!.prepend(button)
  let displayed = false
  const show = async () => {
    displayed = true
    const content = showPanel(tr('Assistance à la rédaction'))
    if (!isTauri) {
      content.textContent = tr('La connexion à un agent est disponible dans l’application Tauri.')
      return
    }
    const state = await transport.invoke<{ connected: boolean; proposal: Proposal | null }>(
      'agent_state',
    )
    if (!state.connected) {
      const paragraph = document.createElement('p')
      paragraph.textContent = tr(
        'Raccordez votre agent à ce document. Ses propositions seront soumises à votre validation avant toute modification.',
      )
      const connect = document.createElement('button')
      connect.className = 'primary'
      connect.textContent = tr('Autoriser une connexion MCP')
      connect.onclick = () => {
        void transport.invoke<Record<string, unknown>>('agent_invite').then((info) => {
          const pre = document.createElement('pre')
          pre.textContent = JSON.stringify({ command: info.binary, args: info.arguments }, null, 2)
          content.replaceChildren(pre)
          const text = document.createElement('p')
          text.textContent = tr(
            'Configuration fixe : reconnectez le serveur dans votre outil après une nouvelle invitation.',
          )
          content.append(text)
        })
      }
      content.append(paragraph, connect)
      return
    }
    const revoke = document.createElement('button')
    revoke.textContent = tr('Déconnecter l’agent')
    revoke.onclick = () => {
      void transport.invoke('agent_revoke').then(show)
    }
    content.append(revoke)
    const p = state.proposal
    if (!p) {
      const text = document.createElement('p')
      text.textContent = tr(
        'Connexion autorisée. L’agent peut lire le document publié et préparer un lot de modifications.',
      )
      content.append(text)
      return
    }
    const heading = document.createElement('h3')
    heading.textContent = p.summary
    content.append(heading)
    const prose = (node: JSONContent): string =>
      node.text ||
      node.attrs?.text ||
      node.content?.map(prose).join(node.type === 'doc' ? '\n\n' : '') ||
      ''
    for (const [label, document] of [
      [tr('Actuel'), p.before],
      [tr('Proposition'), p.after],
    ] as const) {
      const section = window.document.createElement('section')
      const title = window.document.createElement('h4')
      title.textContent = label
      const text = window.document.createElement('div')
      text.className = 'proposal-text'
      text.textContent = prose(document)
      section.append(title, text)
      content.append(section)
    }
    if (p.status !== 'pending' || p.revision !== provider.revision) {
      const status = document.createElement('p')
      status.className = 'error'
      status.textContent =
        p.status === 'accepted'
          ? tr('Proposition acceptée.')
          : p.status === 'rejected'
            ? tr('Proposition refusée.')
            : tr('Proposition périmée ou terminée. Le document doit être relu par l’agent.')
      content.append(status)
      return
    }
    const actions = document.createElement('div')
    actions.className = 'panel-actions'
    const reject = document.createElement('button')
    reject.textContent = tr('Refuser le lot')
    reject.onclick = () => {
      void transport.invoke('agent_reject', { id: p.id }).then(show)
    }
    const accept = document.createElement('button')
    accept.className = 'primary'
    accept.textContent = tr('Accepter le lot')
    accept.onclick = () => {
      void (async () => {
        if (editor.view.composing)
          throw new Error(tr('Terminez la saisie en cours avant de décider.'))
        await provider.flush()
        if (provider.revision !== p.revision) throw new Error(tr('Proposition périmée.'))
        const update = prepareProposal(provider, p.stages)
        yUndoPluginKey.getState(editor.state)?.undoManager.stopCapturing()
        await transport.invoke('agent_accept', { id: p.id, update: Array.from(update) })
        message(tr('Lot accepté. Vous pouvez l’annuler en une seule fois.'))
        await show()
      })().catch((error) => message(String(error), true))
    }
    actions.append(reject, accept)
    content.append(actions)
  }
  button.onclick = () => {
    void show().catch((error) => message(String(error), true))
  }
  void transport.listen('agent:changed', () => {
    button.classList.add('has-proposal')
    if (displayed && !document.querySelector<HTMLElement>('.side-panel')!.hidden) void show()
  })
}

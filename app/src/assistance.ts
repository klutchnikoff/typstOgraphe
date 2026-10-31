import { comparisonAPI } from './comparison-api'
import { proposalDialog } from './version-dialog'
import type { Catalogue } from './shell'
import { actionRow } from './keyboard-navigation'
import { tr } from './i18n'
import { icon } from './icons'
import { type Editor, type JSONContent } from '@tiptap/core'

import type { Provider, Transport } from './provider'
interface Proposal {
  id: string
  summary: string
  revision: number
  status: string
  before: JSONContent
  after: JSONContent
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
  let currentContent: HTMLElement | undefined
  let request = 0
  let viewer: ReturnType<typeof proposalDialog> | undefined
  const show = async () => {
    const ticket = ++request
    const host = showPanel(tr('Assistance à la rédaction'))
    const content = document.createElement('div')
    content.className = 'assistance-panel'
    host.append(content)
    currentContent = content
    if (!isTauri) {
      content.textContent = tr('La connexion à un agent est disponible dans l’application Tauri.')
      return
    }
    const state = await transport.invoke<{ connected: boolean; proposal: Proposal | null }>(
      'agent_state',
    )
    if (ticket !== request || !content.isConnected) return
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
      const actions = actionRow()
      actions.append(connect)
      content.append(paragraph, actions)
      return
    }
    const revoke = document.createElement('button')
    revoke.className = 'danger'
    revoke.textContent = tr('Déconnecter l’agent')
    revoke.onclick = () => {
      void transport.invoke('agent_revoke').then(show)
    }
    const connectionActions = actionRow()
    connectionActions.append(revoke)
    content.append(connectionActions)
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
    const review = document.createElement('button')
    review.className = 'primary'
    review.textContent = tr('Examiner la proposition')
    review.onclick = () => {
      void (async () => {
        if (editor.view.composing)
          throw new Error(tr('Terminez la saisie en cours avant de décider.'))
        const catalogue = await transport.invoke<Catalogue>('content_catalogue')
        viewer?.destroy()
        viewer = proposalDialog({
          id: p.id,
          title: p.summary,
          api: comparisonAPI(provider, transport),
          catalogue,
          returnFocus: () => (review.isConnected ? review : button),
        })
      })().catch((error) => message(String(error), true))
    }
    const actions = actionRow()
    const reject = document.createElement('button')
    reject.textContent = tr('Refuser le lot')
    reject.onclick = () => {
      void transport
        .invoke('agent_reject', { id: p.id })
        .then(show)
        .catch((error) => message(String(error), true))
    }
    reject.hidden = p.status !== 'pending'
    actions.append(reject, review)
    content.append(actions)
  }
  button.onclick = () => {
    void show().catch((error) => message(String(error), true))
  }
  void transport.listen('agent:changed', () => {
    button.classList.add('has-proposal')
    if (currentContent?.isConnected && !document.querySelector<HTMLElement>('.side-panel')!.hidden)
      void show().catch((error) => message(String(error), true))
  })
}

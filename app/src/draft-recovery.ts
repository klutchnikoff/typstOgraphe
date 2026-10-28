import type { Provider } from './provider'
import { actionRow } from './keyboard-navigation'
import { tr } from './i18n'

/** Refusal is explicit. Never merge a rejected replica back into confirmed state. */
export function draftRecovery(provider: Provider, content: () => unknown, native: boolean) {
  const band = document.createElement('section')
  band.className = 'draft-recovery'
  band.setAttribute('role', 'region')
  band.setAttribute('aria-label', tr('Brouillon non synchronisé'))
  band.hidden = true
  const explanation = document.createElement('p')
  explanation.setAttribute('role', 'status')
  explanation.textContent = tr('La synchronisation est suspendue ; votre brouillon est conservé.')
  const actions = actionRow()
  const discard = document.createElement('button')
  discard.textContent = tr('Revenir à la version confirmée')
  const download = document.createElement('button')
  download.className = 'primary'
  download.textContent = tr('Exporter le brouillon')
  download.onclick = () => {
    const draft = { ...provider.draft(), content: content() }
    if (native) {
      void provider.transport.invoke('export_editing_draft', { draft }).catch(provider.onError)
      return
    }
    const url = URL.createObjectURL(new Blob([JSON.stringify(draft)], { type: 'application/json' }))
    const link = document.createElement('a')
    link.href = url
    link.download = 'typstographe-draft.json'
    link.click()
    URL.revokeObjectURL(url)
  }
  const confirmation = document.createElement('dialog')
  confirmation.className = 'app-dialog'
  const question = document.createElement('p')
  question.id = 'discard-draft-question'
  question.textContent = tr(
    'Abandonner ce brouillon et recharger la version confirmée ? Exportez-le d’abord si vous souhaitez le conserver.',
  )
  confirmation.setAttribute('aria-labelledby', question.id)
  const controls = actionRow()
  const cancel = document.createElement('button')
  cancel.textContent = tr('Annuler')
  cancel.onclick = () => confirmation.close()
  const confirm = document.createElement('button')
  confirm.textContent = tr('Abandonner le brouillon')
  confirm.onclick = () => {
    confirm.disabled = true
    void provider
      .discardDraft()
      .then(() => location.reload())
      .catch((error) => {
        confirm.disabled = false
        provider.onError(error)
      })
  }
  controls.append(confirm, cancel)
  confirmation.append(question, controls)
  confirmation.onclose = () => {
    confirmation.remove()
    discard.focus()
  }
  discard.onclick = () => {
    document.body.append(confirmation)
    confirmation.showModal()
    cancel.focus()
  }
  actions.append(discard, download)
  band.append(explanation, actions)
  document.querySelector('.writing')?.prepend(band)
  if (!band.isConnected) document.querySelector('#editor')?.before(band)
  return {
    refresh: () => {
      band.hidden = !provider.rejected
    },
  }
}

import { actionRow, keyboardScope } from './keyboard-navigation'
import { tr } from './i18n'
export type CloseChoice = 'clean' | 'saved' | 'keep' | 'discard'
/** The same dialog governs window close, document replacement and quit. */
export async function decideClose(options: {
  dirty: boolean
  pending: number
  otherWindows?: number
  save: () => Promise<boolean>
  keep: () => Promise<void>
  discard: () => Promise<void>
}): Promise<CloseChoice | null> {
  const privateWork = !!(options.pending || options.otherWindows)
  if (!options.dirty && !privateWork) return 'clean'
  const previous = document.activeElement as HTMLElement | null
  const dialog = document.createElement('dialog')
  dialog.className = 'app-dialog recovery-close-dialog'
  const title = document.createElement('h2'),
    text = document.createElement('p'),
    error = document.createElement('p'),
    actions = actionRow()
  title.id = 'recovery-close-title'
  dialog.setAttribute('aria-labelledby', title.id)
  error.role = 'alert'
  title.textContent = tr(
    privateWork ? 'Conserver le travail avant de fermer ?' : 'Enregistrer les modifications ?',
  )
  text.textContent = privateWork
    ? `${tr('Le document et les saisies en cours seront conservés sur cet ordinateur.')} ${tr('Le fichier .typsto ne sera pas modifié.')}`
    : tr('Ce document contient des modifications qui ne sont pas encore enregistrées.')
  if (options.pending) {
    const count = document.createElement('span')
    count.textContent = `${tr('Saisies à reprendre')} : ${options.pending}`
    text.append(document.createElement('br'), count)
  }
  if (options.otherWindows) {
    const scope = document.createElement('p')
    scope.textContent = tr(
      'Les autres fenêtres seront fermées. Leurs saisies en cours seront conservées.',
    )
    text.after(scope)
    dialog.append(scope)
  }
  const abandon = document.createElement('button'),
    cancel = document.createElement('button'),
    primary = document.createElement('button')
  abandon.className = 'danger'
  abandon.textContent = tr(privateWork ? 'Abandonner et fermer' : 'Fermer sans enregistrer')
  cancel.textContent = tr('Annuler')
  primary.className = 'primary'
  primary.dataset.default = ''
  primary.textContent = tr(privateWork ? 'Conserver et fermer' : 'Enregistrer et fermer')
  actions.append(abandon, cancel, primary)
  dialog.prepend(title, text)
  dialog.append(error, actions)
  document.body.append(dialog)
  return new Promise((resolve) => {
    let busy = false
    const finish = (choice: CloseChoice | null) => {
      dialog.close()
      dialog.remove()
      if (!choice)
        setTimeout(() => {
          if (
            previous?.isConnected &&
            (document.activeElement === document.body ||
              document.activeElement?.closest('.recovery-close-dialog'))
          )
            previous.focus({ preventScroll: true })
        }, 0)
      resolve(choice)
    }
    const run = async (choice: CloseChoice) => {
      if (busy) return
      busy = true
      actions.querySelectorAll('button').forEach((b) => (b.disabled = true))
      error.textContent = ''
      try {
        if (choice === 'discard') await options.discard()
        else if (choice === 'keep') await options.keep()
        else if (!(await options.save())) {
          busy = false
          finish(null)
          return
        }
        finish(choice)
      } catch (reason) {
        error.textContent = String(reason)
        primary.textContent = tr('Réessayer')
        busy = false
        actions.querySelectorAll('button').forEach((b) => (b.disabled = false))
      }
    }
    abandon.onclick = () => void run('discard')
    primary.onclick = () => void run(privateWork ? 'keep' : 'saved')
    cancel.onclick = () => finish(null)
    dialog.addEventListener('cancel', (event) => {
      event.preventDefault()
      if (!busy) finish(null)
    })
    keyboardScope(dialog, {
      close: () => {
        if (!busy) finish(null)
      },
      confirm: () => primary.click(),
      canConfirm: () => !busy,
      tab: true,
      tabLoop: true,
      priority: 50,
    })
    dialog.showModal()
    primary.focus()
  })
}

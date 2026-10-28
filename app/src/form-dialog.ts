import { contextPanel } from './context-panel'
import { tr } from './i18n'

/** Reusable modal shell. Field rendering and draft publication belong to its caller. */
export function formDialog(options: {
  id: string
  title: string
  cancel: () => void
  returnFocus: () => HTMLElement | undefined
}) {
  const element = document.createElement('dialog')
  element.className = 'app-dialog form-dialog'
  const form = contextPanel({
    id: options.id,
    closeLabel: tr('Annuler'),
    close: options.cancel,
    canConfirm: () => true,
    modal: true,
  })
  form.title.textContent = options.title
  form.element.className = 'dialog-form'
  form.element.hidden = false
  element.setAttribute('aria-labelledby', form.title.id)
  element.append(form.element)
  element.addEventListener('cancel', (event) => {
    event.preventDefault()
    options.cancel()
  })
  // WebKit releases native modality after close(). Restore focus once that
  // lifecycle has finished, including when the original add button disappeared.
  element.addEventListener('close', () => {
    const target = options.returnFocus()
    if (
      target?.isConnected &&
      !document.querySelector('dialog[open]') &&
      (document.activeElement === document.body || element.contains(document.activeElement))
    )
      target.focus({ preventScroll: true })
  })
  return {
    ...form,
    dialog: element,
    open() {
      document.body.append(element)
      element.showModal()
      form.focus()
    },
    destroy() {
      if (element.open) element.close()
      element.remove()
    },
  }
}

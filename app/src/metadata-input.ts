import { tr } from './i18n'
import type { FieldTarget, MetadataCommands } from './metadata-commands'
import { actionRow } from './keyboard-navigation'

type Control = HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
const bindings = new WeakMap<Control, { refresh: () => void; destroy: () => void }>()
/** Same pending value, IME isolation, refusal and explicit retry/discard for every native field. */
export function bindMetadataInput(
  input: Control,
  commands: MetadataCommands,
  target: FieldTarget,
  read: () => unknown,
  parse: () => unknown = () => input.value,
  format: (value: unknown) => string = (value) => String(value ?? ''),
) {
  let composing = false
  const status = document.createElement('div')
  const error = document.createElement('p')
  error.role = 'alert'
  status.hidden = true
  const retry = document.createElement('button'),
    cancel = document.createElement('button')
  retry.type = cancel.type = 'button'
  retry.textContent = tr('Valider')
  cancel.textContent = tr('Annuler')
  retry.onclick = () => commands.set(target, parse(), read)
  cancel.onclick = () => {
    commands.discard(target)
    input.focus()
  }
  const actions = actionRow()
  retry.className = 'primary'
  actions.append(cancel, retry)
  status.append(error, actions)
  ;(input.closest('.field') || input.parentElement!.parentElement!).append(status)
  const update = () => {
    if (!composing) commands.set(target, parse(), read)
  }
  const begin = () => {
    composing = true
    commands.composing(input, true)
  }
  const end = () => {
    composing = false
    commands.composing(input, false)
    update()
  }
  const refresh = () => {
    const reason = commands.error(target)
    status.hidden = !reason
    error.textContent = reason?.message || ''
    input.setAttribute('aria-invalid', String(!!reason))
    if (composing) return
    const value = commands.value(target, read())
    if (input instanceof HTMLInputElement && input.type === 'checkbox')
      input.checked = Boolean(value)
    else {
      const text = format(value)
      // A trailing empty keyword line is local input, not another keyword.
      if (
        input.value === text ||
        (input.dataset.fieldKind === 'tokens' && document.activeElement === input && !reason)
      )
        return
      const start = input instanceof HTMLSelectElement ? null : input.selectionStart,
        finish = input instanceof HTMLSelectElement ? null : input.selectionEnd
      input.value = text
      if (
        document.activeElement === input &&
        start !== null &&
        finish !== null &&
        !(input instanceof HTMLSelectElement)
      )
        input.setSelectionRange(Math.min(start, text.length), Math.min(finish, text.length))
    }
  }
  input.addEventListener('input', update)
  input.addEventListener('compositionstart', begin)
  input.addEventListener('compositionend', end)
  const unsubscribe = commands.watch(refresh)
  const binding = {
    refresh,
    destroy() {
      unsubscribe()
      input.removeEventListener('input', update)
      input.removeEventListener('compositionstart', begin)
      input.removeEventListener('compositionend', end)
      if (composing) commands.composing(input, false)
      bindings.delete(input)
      status.remove()
    },
  }
  bindings.set(input, binding)
  refresh()
  return binding
}
export const metadataInputBinding = (input: Control) => bindings.get(input)

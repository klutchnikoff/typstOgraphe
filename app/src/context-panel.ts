import { trackForm } from './form-drafts'
import { actionRow, initialNavigationGroup, keyboardScope, visible } from './keyboard-navigation'
/** Shared panel anatomy. Only body and actions belong to a capability/corrector. */
export function contextPanel(options: {
  id: string
  closeLabel: string
  close: () => void
  canConfirm: () => boolean
  tabExit?: () => void
  titleId?: string
  closeId?: string
  errorId?: string
  modal?: boolean
}) {
  const element = document.createElement('form')
  element.id = options.id
  element.tabIndex = -1
  element.noValidate = true
  element.className = 'properties'
  element.hidden = true
  const header = document.createElement('div')
  header.className = 'panel-title'
  const title = document.createElement('strong')
  title.id = options.titleId || `${options.id}-title`
  element.setAttribute('aria-labelledby', title.id)
  const close = document.createElement('button')
  close.type = 'button'
  close.textContent = '×'
  close.setAttribute('aria-label', options.closeLabel)
  if (options.closeId) close.id = options.closeId
  close.onclick = options.close
  header.append(title, close)
  const body = document.createElement('div')
  body.className = 'context-panel-body'
  const actions = actionRow()
  const error = document.createElement('p')
  error.setAttribute('role', 'alert')
  if (options.errorId) error.id = options.errorId
  element.append(header, body, actions, error)
  let recovery: ReturnType<typeof trackForm> | undefined
  let compositions = 0
  element.addEventListener(
    'compositionstart',
    () => {
      compositions++
    },
    true,
  )
  element.addEventListener(
    'compositionend',
    () => {
      compositions = Math.max(0, compositions - 1)
    },
    true,
  )
  let preferred: HTMLButtonElement | undefined
  const panel = {
    element,
    recoveryData: (): unknown => null,
    trackRecovery(owner: object, context: unknown) {
      recovery?.detach()
      recovery = trackForm(owner, element, context, () => panel.recoveryData())
    },
    discardRecovery() {
      recovery?.discard()
      recovery = undefined
    },
    detachRecovery() {
      recovery?.detach()
      recovery = undefined
    },
    draftChanged() {
      recovery?.changed()
    },
    title,
    body,
    actions,
    error,
    focus() {
      const eligible = (selector: string) =>
        [...element.querySelectorAll<HTMLElement>(selector)].find(
          (control) => visible(control) && !control.matches(':disabled, [aria-disabled="true"]'),
        )
      const target =
        eligible('[data-initial-focus]') ||
        eligible('[aria-checked="true"]') ||
        eligible(
          '.context-panel-body input, .context-panel-body textarea, .context-panel-body select',
        ) ||
        element
      target.focus({ preventScroll: true })
    },
    confirm: () => preferred?.click(),
    submit() {
      if (!compositions && options.canConfirm()) panel.confirm()
    },
    action(options: {
      id?: string
      label: string
      run: () => void
      danger?: boolean
      primary?: boolean
      parent?: HTMLElement
    }) {
      const button = document.createElement('button')
      button.type = 'button'
      button.textContent = options.label
      if (options.id) button.id = options.id
      button.classList.toggle('danger', !!options.danger)
      button.onclick = () => {
        if (!options.primary || !compositions) options.run()
      }
      ;(options.parent || actions).append(button)
      if (options.primary) panel.setDefault(button)
      return button
    },
    setDefault(button: HTMLButtonElement) {
      if (preferred) {
        preferred.classList.remove('primary')
        delete preferred.dataset.default
      }
      preferred = button
      button.classList.add('primary')
      button.dataset.default = ''
    },
  }
  keyboardScope(element, {
    close: options.close,
    confirm: () => panel.submit(),
    canConfirm: options.canConfirm,
    tab: true,
    tabLoop: options.modal,
    modifiedEnter: options.modal,
    tabExit: options.tabExit,
    enterMultiline: !options.modal,
    priority: options.modal ? 40 : 20,
    initialGroup: () => initialNavigationGroup(element),
  })
  element.onsubmit = (event) => {
    event.preventDefault()
    panel.submit()
  }
  return panel
}

export type ContextPanel = ReturnType<typeof contextPanel>

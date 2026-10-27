import { actionRow, keyboardScope } from './keyboard-navigation'
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
  let preferred: HTMLButtonElement | undefined
  const panel = {
    element,
    title,
    body,
    actions,
    error,
    setEmbedded(embedded: boolean) {
      header.hidden = embedded
      element.classList.toggle('widget-context-panel', embedded)
    },
    confirm: () => preferred?.click(),
    submit() {
      if (options.canConfirm()) panel.confirm()
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
      button.onclick = options.run
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
    tabExit: options.tabExit,
    enterMultiline: true,
    priority: 20,
    initialGroup: () => body.querySelector('[data-default-navigation]'),
  })
  element.onsubmit = (event) => {
    event.preventDefault()
    panel.submit()
  }
  return panel
}

export type ContextPanel = ReturnType<typeof contextPanel>

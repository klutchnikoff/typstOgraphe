import type { PublicCommand, Registry } from './commands'
import { tr } from './i18n'

let nextId = 0

/** The input owns focus; aria-activedescendant exposes the keyboard choice. */
export function commandPalette(
  content: HTMLElement,
  commands: Registry,
  execute: (command: PublicCommand) => void,
) {
  const input = document.createElement('input')
  input.placeholder = tr('Rechercher une commande…')
  input.setAttribute('aria-label', tr('Rechercher une commande'))
  input.setAttribute('role', 'combobox')
  input.setAttribute('aria-autocomplete', 'list')
  input.setAttribute('aria-expanded', 'true')
  const results = document.createElement('div')
  results.className = 'command-palette-results'
  results.id = `command-palette-${++nextId}`
  results.setAttribute('role', 'listbox')
  results.setAttribute('aria-label', tr('Commandes'))
  input.setAttribute('aria-controls', results.id)
  const status = document.createElement('p')
  status.setAttribute('role', 'status')
  let rows: { command: PublicCommand; button: HTMLButtonElement }[] = []
  let selected = -1
  const enabled = (index: number) => rows[index]?.command.enabled?.() !== false && !!rows[index]
  const select = (index: number, reveal = false) => {
    selected = index
    rows.forEach(({ button }, position) =>
      button.setAttribute('aria-selected', String(position === index)),
    )
    const button = rows[index]?.button
    if (button) {
      input.setAttribute('aria-activedescendant', button.id)
      if (reveal)
        button.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' })
    } else input.removeAttribute('aria-activedescendant')
  }
  const refreshAvailability = () => {
    rows.forEach(({ command, button }) => {
      button.disabled = command.enabled?.() === false
      button.setAttribute('aria-disabled', String(button.disabled))
    })
    if (!enabled(selected)) select(-1)
  }
  const fill = () => {
    results.replaceChildren()
    rows = []
    const query = input.value.toLocaleLowerCase()
    for (const command of commands.values()) {
      if (!command.label.toLocaleLowerCase().includes(query)) continue
      const button = document.createElement('button')
      button.type = 'button'
      button.id = `${results.id}-option-${rows.length}`
      button.tabIndex = -1
      button.setAttribute('role', 'option')
      button.textContent = command.label + (command.shortcut ? ` · ${command.shortcut}` : '')
      button.onmousedown = (event) => event.preventDefault()
      button.onclick = () => {
        refreshAvailability()
        if (command.enabled?.() !== false) execute(command)
      }
      rows.push({ command, button })
      results.append(button)
    }
    selected = -1
    refreshAvailability()
    select(rows.findIndex((_, index) => enabled(index)))
    results.scrollTop = 0
    status.textContent = rows.length ? '' : tr('Aucune commande ne correspond.')
    status.hidden = !!rows.length
  }
  input.oninput = fill
  input.onkeydown = (event) => {
    // Native IME processing events must not execute a command when confirming text.
    if (
      event.isComposing ||
      event.keyCode === 229 ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey
    )
      return
    if (!['ArrowDown', 'ArrowUp', 'Enter'].includes(event.key)) return
    event.preventDefault()
    event.stopPropagation()
    refreshAvailability()
    if (event.key === 'Enter') {
      if (enabled(selected)) execute(rows[selected].command)
      return
    }
    const candidates = rows.flatMap((_, index) => (enabled(index) ? [index] : []))
    if (!candidates.length) return
    const current = candidates.indexOf(selected)
    const index =
      current < 0
        ? event.key === 'ArrowDown'
          ? 0
          : candidates.length - 1
        : (current + (event.key === 'ArrowDown' ? 1 : -1) + candidates.length) % candidates.length
    select(candidates[index], true)
  }
  content.append(input, results, status)
  fill()
  input.focus()
}

import { navigationGroup } from './keyboard-navigation'
import type { PublicCommand, Registry } from './commands'
import { commandAvailability, refreshCommandButton } from './public-commands'
import { tr } from './i18n'

let nextId = 0

/** The input owns focus; aria-activedescendant exposes the keyboard choice. */
export function commandPalette(
  content: HTMLElement,
  commands: Registry,
  execute: (command: PublicCommand) => void,
) {
  const input = document.createElement('input')
  input.className = 'command-palette-input'
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
  const navigation = navigationGroup(results, {
    axis: 'vertical',
    input,
    beforeNavigate: () => refreshAvailability(),
    onActivate: (button) => {
      const row = rows.find((row) => row.button === button)
      if (row && commandAvailability(row.command).enabled) execute(row.command)
    },
  })
  const refreshAvailability = () => {
    rows.forEach(({ command, button }) => {
      refreshCommandButton(button, command)
    })
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
      rows.push({ command, button })
      results.append(button)
    }
    refreshAvailability()
    navigation.refresh(true)
    results.scrollTop = 0
    status.textContent = rows.length ? '' : tr('Aucune commande ne correspond.')
    status.hidden = !!rows.length
  }
  const refresh = () => {
    if (!input.isConnected) {
      document.removeEventListener('commands:availability', refresh)
      return
    }
    refreshAvailability()
    navigation.refresh()
  }
  document.addEventListener('commands:availability', refresh)
  input.oninput = fill
  content.append(input, results, status)
  fill()
  navigation.refresh(true)
  input.focus()
}

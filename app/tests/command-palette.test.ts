import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { commandPalette } from '../src/command-palette'
import type { PublicCommand, Registry } from '../src/commands'

const originalScroll = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView')
beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  })
})
afterEach(() => {
  document.body.replaceChildren()
  if (originalScroll) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', originalScroll)
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView')
})
function mount(entries: { id: string; label: string; enabled?: () => boolean }[]) {
  const content = document.createElement('div')
  document.body.append(content)
  const commands: Registry = new Map(entries.map((entry) => [entry.id, { ...entry, run: vi.fn() }]))
  const execute = vi.fn((command: PublicCommand) => command.run())
  commandPalette(content, commands, execute)
  const input = content.querySelector('input')!
  const key = (value: string, composing = false) =>
    input.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: value,
        isComposing: composing,
        cancelable: true,
        bubbles: true,
      }),
    )
  const query = (value: string) => {
    input.value = value
    input.dispatchEvent(new Event('input'))
  }
  const active = () => content.querySelector('[aria-selected="true"]')?.textContent
  return { content, commands, execute, input, key, query, active }
}

it('parcourt en boucle les propositions actives sans quitter la recherche ni changer sa saisie', () => {
  const palette = mount([
    { id: 'first', label: 'Commande une' },
    { id: 'disabled', label: 'Commande indisponible', enabled: () => false },
    { id: 'last', label: 'Commande deux' },
  ])
  palette.query('Commande')
  palette.input.setSelectionRange(3, 3)
  expect(palette.active()).toBe('Commande une')
  palette.key('ArrowDown')
  expect(palette.active()).toBe('Commande deux')
  expect(document.activeElement).toBe(palette.input)
  expect(palette.input.value).toBe('Commande')
  expect(palette.input.selectionStart).toBe(3)
  expect(palette.key('Tab')).toBe(true)
  const active = palette.content.querySelector('[aria-selected="true"]')!
  expect(palette.input.getAttribute('aria-activedescendant')).toBe(active.id)
  expect(active.scrollIntoView).toHaveBeenCalledWith({
    block: 'nearest',
    inline: 'nearest',
    behavior: 'instant',
  })
  palette.key('ArrowDown')
  expect(palette.active()).toBe('Commande une')
  palette.key('ArrowUp')
  expect(palette.active()).toBe('Commande deux')
  palette.key('Enter', true)
  palette.input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', keyCode: 229 }))
  expect(palette.execute).not.toHaveBeenCalled()
  palette.key('Enter')
  expect(palette.commands.get('last')!.run).toHaveBeenCalledOnce()
})
it('réinitialise le choix au filtrage et ne lance rien si aucun résultat ne correspond', () => {
  const palette = mount([
    { id: 'one', label: 'Titre 1' },
    { id: 'two', label: 'Titre 2' },
  ])
  palette.key('ArrowDown')
  expect(palette.active()).toBe('Titre 2')
  palette.query('1')
  expect(palette.active()).toBe('Titre 1')
  palette.query('aucune')
  expect(palette.content.querySelector('[role="status"]')!.textContent).toBe(
    'Aucune commande ne correspond.',
  )
  expect(palette.input.hasAttribute('aria-activedescendant')).toBe(false)
  palette.key('ArrowDown')
  palette.key('ArrowUp')
  palette.key('Enter')
  expect(palette.execute).not.toHaveBeenCalled()
  palette.query('2')
  palette.key('Enter')
  expect(palette.commands.get('two')!.run).toHaveBeenCalledOnce()
})
it('revérifie la disponibilité avant toute exécution, clavier ou souris', () => {
  let available = true
  const palette = mount([
    { id: 'one', label: 'Une', enabled: () => available },
    { id: 'two', label: 'Deux' },
  ])
  available = false
  palette.key('Enter')
  expect(palette.execute).not.toHaveBeenCalled()
  expect(palette.content.querySelector('[role="option"]')).toHaveProperty('disabled', true)
  palette.key('ArrowDown')
  expect(palette.active()).toBe('Deux')
  ;[...palette.content.querySelectorAll<HTMLButtonElement>('button')][1].click()
  expect(palette.commands.get('two')!.run).toHaveBeenCalledOnce()
  expect(palette.commands.get('one')!.run).not.toHaveBeenCalled()
})

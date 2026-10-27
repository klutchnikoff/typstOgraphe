import { afterEach, expect, it, vi } from 'vitest'
import { navigationGroup } from '../src/keyboard-navigation'
import { contextPanel } from '../src/context-panel'

afterEach(() => document.body.replaceChildren())
function panel() {
  const result = contextPanel({
    id: 'test-band',
    closeLabel: 'Fermer',
    close: vi.fn(),
    canConfirm: () => true,
  })
  result.element.hidden = false
  document.body.append(result.element)
  return result
}
it('un nouveau bandeau choisit automatiquement son premier champ disponible', () => {
  const band = panel()
  const disabled = document.createElement('input'),
    hidden = document.createElement('input'),
    text = document.createElement('textarea')
  disabled.disabled = hidden.hidden = true
  band.body.append(disabled, hidden, text)
  band.focus()
  expect(document.activeElement).toBe(text)
  const replacement = document.createElement('input')
  band.body.replaceChildren(replacement)
  band.focus()
  expect(document.activeElement).toBe(replacement)
})
it('les choix exclusifs et les actions initiales explicites gardent leur priorité', () => {
  const band = panel(),
    input = document.createElement('input'),
    checked = document.createElement('button')
  checked.setAttribute('aria-checked', 'true')
  band.body.append(input, checked)
  band.focus()
  expect(document.activeElement).toBe(checked)
  const action = band.action({ label: 'Action', run: vi.fn() })
  action.dataset.initialFocus = ''
  band.focus()
  expect(document.activeElement).toBe(action)
})
it('un bandeau sans champ garde le focus global pour les suggestions différées', () => {
  const band = panel()
  band.action({ label: 'Ignorer', run: vi.fn() })
  band.focus()
  expect(document.activeElement).toBe(band.element)
})

it('les flèches ignorent les suggestions vides ou désactivées et rejoignent les actions', async () => {
  const original = HTMLElement.prototype.scrollIntoView
  HTMLElement.prototype.scrollIntoView = vi.fn()
  const band = panel(),
    choices = document.createElement('div'),
    run = vi.fn()
  choices.dataset.defaultNavigation = ''
  band.body.append(choices)
  navigationGroup(choices, { axis: 'horizontal' })
  const action = band.action({ label: 'Ignorer', run })
  const arrow = () =>
    document.activeElement!.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'ArrowRight',
        bubbles: true,
        cancelable: true,
      }),
    )
  band.focus()
  arrow()
  expect(document.activeElement).toBe(action)
  expect(run).not.toHaveBeenCalled()
  const suggestion = document.createElement('button')
  suggestion.textContent = 'Correction'
  choices.append(suggestion)
  await Promise.resolve()
  expect(document.activeElement).toBe(action)
  band.focus()
  arrow()
  expect(document.activeElement).toBe(suggestion)
  suggestion.disabled = true
  band.focus()
  arrow()
  expect(document.activeElement).toBe(action)
  expect(run).not.toHaveBeenCalled()
  HTMLElement.prototype.scrollIntoView = original
})

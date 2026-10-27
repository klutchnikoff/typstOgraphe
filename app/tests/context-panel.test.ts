import { afterEach, expect, it, vi } from 'vitest'
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

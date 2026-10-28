import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { actionRow, keyboardScope, navigationGroup } from '../src/keyboard-navigation'
import { installMenus } from '../src/menus'

const scroll = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView')
beforeEach(() =>
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  }),
)
afterEach(() => {
  document.body.replaceChildren()
  if (scroll) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scroll)
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView')
})
const key = (element: HTMLElement, value: string, init: KeyboardEventInit = {}) =>
  element.dispatchEvent(
    new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true, ...init }),
  )
const tick = async () => {
  await Promise.resolve()
  await Promise.resolve()
}
function button(text: string) {
  const element = document.createElement('button')
  element.type = 'button'
  element.textContent = text
  return element
}
it('inclut automatiquement les boutons ajoutés, ignore les boutons masqués/désactivés et ne les active pas en naviguant', async () => {
  const row = actionRow(),
    first = button('Annuler'),
    next = button('Valider'),
    hidden = button('Invisible'),
    disabled = button('Indisponible')
  hidden.hidden = true
  disabled.disabled = true
  row.append(first, hidden, disabled)
  document.body.append(row)
  const click = vi.fn()
  next.onclick = click
  first.focus()
  row.append(next)
  await tick()
  key(first, 'ArrowRight')
  expect(document.activeElement).toBe(next)
  expect(click).not.toHaveBeenCalled()
  key(next, 'ArrowRight')
  expect(document.activeElement).toBe(first)
  key(first, 'End')
  expect(document.activeElement).toBe(next)
})
it('les groupes imbriqués gardent leur axe et laissent remonter l’autre direction', () => {
  const list = document.createElement('div'),
    first = document.createElement('section'),
    second = document.createElement('section')
  first.tabIndex = second.tabIndex = 0
  const actions = actionRow(),
    a = button('Annuler'),
    b = button('Valider')
  actions.append(a, b)
  first.append(actions)
  list.append(first, second)
  document.body.append(list)
  navigationGroup(list, { axis: 'vertical', items: ':scope > section' })
  a.focus()
  key(a, 'ArrowRight')
  expect(document.activeElement).toBe(b)
  key(b, 'ArrowDown')
  expect(document.activeElement).toBe(second)
})
it('la fermeture concerne la portée la plus proche puis restitue le focus au déclencheur', () => {
  const trigger = button('Déclencheur'),
    outer = document.createElement('section'),
    inner = document.createElement('section'),
    field = document.createElement('input')
  document.body.append(trigger, outer)
  outer.append(inner)
  inner.append(field)
  const closeOuter = vi.fn()
  keyboardScope(outer, { close: closeOuter, priority: 10 })
  const closeInner = vi.fn(() => {
    inner.hidden = true
    trigger.focus()
  })
  keyboardScope(inner, { close: closeInner, priority: 20 })
  field.focus()
  key(field, 'Escape')
  expect(closeInner).toHaveBeenCalledOnce()
  expect(closeOuter).not.toHaveBeenCalled()
  expect(document.activeElement).toBe(trigger)
  key(trigger, 'Escape')
  expect(closeOuter).toHaveBeenCalledOnce()
})
it('un formulaire garde les touches des champs natifs et ne confirme pas pendant une préédition', () => {
  const form = document.createElement('form'),
    input = document.createElement('input'),
    select = document.createElement('select'),
    multiline = document.createElement('textarea'),
    confirm = vi.fn()
  form.append(input, select, multiline)
  document.body.append(form)
  navigationGroup(form, { axis: 'horizontal' })
  keyboardScope(form, { confirm })
  expect(key(input, 'ArrowRight')).toBe(true)
  expect(key(select, 'ArrowDown')).toBe(true)
  key(select, 'Enter')
  key(multiline, 'Enter')
  key(input, 'Enter', { keyCode: 229 })
  expect(confirm).not.toHaveBeenCalled()
  key(input, 'Enter')
  expect(confirm).toHaveBeenCalledOnce()
})
it('une portée peut adresser les choix arrivés après son ouverture sans voler le focus', async () => {
  const form = document.createElement('form'),
    group = document.createElement('div')
  form.tabIndex = -1
  form.append(group)
  document.body.append(form)
  navigationGroup(group, { axis: 'both', roving: true })
  keyboardScope(form, { initialGroup: () => group })
  form.focus()
  const first = button('Une'),
    second = button('Deux')
  first.dataset.default = ''
  group.append(first, second)
  await tick()
  expect(document.activeElement).toBe(form)
  key(form, 'ArrowRight')
  expect(document.activeElement).toBe(second)
  expect(key(second, 'Tab')).toBe(true)
})

it('un dialogue boucle Tab, garde les retours multilignes et valide explicitement sans intervenir pendant l’IME', () => {
  const root = document.createElement('form'),
    input = document.createElement('input'),
    textarea = document.createElement('textarea'),
    cancel = button('Annuler'),
    validate = button('Valider'),
    confirm = vi.fn()
  root.append(input, textarea, cancel, validate)
  document.body.append(root)
  keyboardScope(root, { tab: true, tabLoop: true, modifiedEnter: true, confirm })
  input.focus()
  key(input, 'Tab', { shiftKey: true })
  expect(document.activeElement).toBe(validate)
  key(validate, 'Tab')
  expect(document.activeElement).toBe(input)
  expect(key(textarea, 'Enter')).toBe(true)
  key(textarea, 'Enter', { ctrlKey: true, isComposing: true })
  key(textarea, 'Enter', { metaKey: true, keyCode: 229 })
  expect(confirm).not.toHaveBeenCalled()
  key(textarea, 'Enter', { ctrlKey: true })
  key(textarea, 'Enter', { metaKey: true })
  expect(confirm).toHaveBeenCalledTimes(2)
})
it('le retrait du bouton focalisé révèle automatiquement le suivant', async () => {
  const row = actionRow(),
    a = button('Un'),
    b = button('Deux')
  row.append(a, b)
  document.body.append(row)
  a.focus()
  a.remove()
  await tick()
  expect(document.activeElement).toBe(b)
})
it('les menus créés après installation prennent les flèches et rendent le focus avec Échap', async () => {
  const dispose = installMenus()
  try {
    const menu = document.createElement('details')
    menu.className = 'tool-menu'
    const summary = document.createElement('summary'),
      panel = document.createElement('div'),
      a = button('Un'),
      b = button('Deux')
    summary.textContent = 'Menu'
    panel.className = 'tool-menu-content'
    panel.append(a, b)
    menu.append(summary, panel)
    document.body.append(menu)
    await tick()
    summary.focus()
    key(summary, 'ArrowDown')
    expect(menu.open).toBe(true)
    expect(document.activeElement).toBe(a)
    key(a, 'ArrowDown')
    expect(document.activeElement).toBe(b)
    key(b, 'Escape')
    expect(menu.open).toBe(false)
    expect(document.activeElement).toBe(summary)
  } finally {
    dispose()
  }
})

it('une liste avec focus virtuel ne rejoue pas récursivement un clic et remplace son contrat de formulaire', () => {
  const root = document.createElement('div'),
    input = document.createElement('input'),
    item = button('Choix')
  root.append(item)
  document.body.append(input, root)
  const click = vi.fn()
  item.onclick = click
  navigationGroup(root, { axis: 'vertical', input })
  item.click()
  expect(click).toHaveBeenCalledOnce()
  expect(document.activeElement).toBe(input)
  key(input, 'Enter')
  expect(click).toHaveBeenCalledTimes(2)
  const previous = vi.fn(),
    next = vi.fn()
  keyboardScope(root, { confirm: previous })
  keyboardScope(root, { confirm: next })
  key(root, 'Enter')
  expect(previous).not.toHaveBeenCalled()
  expect(next).toHaveBeenCalledOnce()
})
it('Gauche et Droite passent au menu voisin sans exécuter ses commandes', async () => {
  const bar = document.createElement('div')
  navigationGroup(bar, { axis: 'horizontal' })
  const menus = ['Fichier', 'Édition'].map((label) => {
    const details = document.createElement('details'),
      summary = document.createElement('summary'),
      panel = document.createElement('div')
    details.className = 'tool-menu'
    summary.textContent = label
    panel.className = 'tool-menu-content'
    panel.append(button('Commande'))
    details.append(summary, panel)
    bar.append(details)
    return details
  })
  document.body.append(bar)
  const dispose = installMenus()
  try {
    await tick()
    const click = vi.fn()
    menus[1].querySelector('button')!.onclick = click
    key(menus[0].querySelector('summary')!, 'ArrowDown')
    key(menus[0].querySelector('button')!, 'ArrowRight')
    expect(menus[0].open).toBe(false)
    expect(menus[1].open).toBe(true)
    expect(document.activeElement).toBe(menus[1].querySelector('button'))
    expect(click).not.toHaveBeenCalled()
  } finally {
    dispose()
  }
})

it('Échap ferme le menu puis le panneau avant que l’éditeur ne consomme la touche', async () => {
  const panel = document.createElement('aside'),
    editor = document.createElement('div'),
    menu = document.createElement('details')
  editor.contentEditable = 'true'
  menu.className = 'tool-menu'
  menu.innerHTML =
    '<summary>Menu</summary><div class="tool-menu-content"><button>Action</button></div>'
  panel.append(menu)
  document.body.append(panel, editor)
  const close = vi.fn()
  keyboardScope(panel, { close, priority: 10 })
  const consumed = vi.fn((event: KeyboardEvent) => {
    event.preventDefault()
    event.stopPropagation()
  })
  editor.addEventListener('keydown', consumed)
  const dispose = installMenus()
  try {
    await tick()
    menu.open = true
    editor.focus()
    key(editor, 'Escape')
    expect(menu.open).toBe(false)
    expect(close).not.toHaveBeenCalled()
    editor.focus()
    key(editor, 'Escape')
    expect(close).toHaveBeenCalledOnce()
    expect(consumed).not.toHaveBeenCalled()
  } finally {
    dispose()
  }
})

it('Entrée valide le bandeau depuis un choix radio, sans seulement réactiver ce choix', () => {
  const panel = document.createElement('form'),
    choice = button('Une option'),
    confirm = vi.fn(),
    select = vi.fn()
  choice.role = 'radio'
  choice.onclick = select
  panel.append(choice)
  document.body.append(panel)
  keyboardScope(panel, { confirm })
  choice.focus()
  key(choice, 'Enter')
  expect(confirm).toHaveBeenCalledOnce()
  expect(select).not.toHaveBeenCalled()
})

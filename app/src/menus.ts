import { composing, editingControl, navigateGroup, navigationGroup } from './keyboard-navigation'

/** Every .tool-menu, including menus created later, receives the same contract. */
export function installMenus(root: Document = document) {
  const groups = new WeakMap<HTMLDetailsElement, ReturnType<typeof navigationGroup>>()
  const setup = () => {
    for (const menu of root.querySelectorAll<HTMLDetailsElement>('.tool-menu')) {
      const summary = menu.querySelector<HTMLElement>(':scope > summary'),
        panel = menu.querySelector<HTMLElement>(':scope > .tool-menu-content')
      if (!summary || !panel) continue
      summary.setAttribute('aria-haspopup', 'menu')
      summary.setAttribute('aria-expanded', String(menu.open))
      panel.setAttribute('role', 'menu')
      let navigation = groups.get(menu)
      if (!navigation) {
        navigation = navigationGroup(panel, {
          axis: 'vertical',
          roving: true,
          itemRole: 'menuitem',
        })
        groups.set(menu, navigation)
      }
      navigation.refresh()
    }
  }
  const menus = () => root.querySelectorAll<HTMLDetailsElement>('.tool-menu[open]')
  const dismissOutside = (event: Event) => {
    const target = event.target
    if (!(target instanceof Element)) return
    const inside = target.closest('.tool-menu')
    for (const menu of menus()) if (menu !== inside) menu.open = false
  }
  const keyboard = (event: KeyboardEvent) => {
    if (composing(event) || event.ctrlKey || event.metaKey || event.altKey) return
    const target = event.target instanceof Element ? event.target : null
    const summary = target?.closest('summary')
    const menu = summary?.closest<HTMLDetailsElement>('.tool-menu')
    if (menu && ['ArrowDown', 'ArrowUp'].includes(event.key)) {
      setup()
      event.preventDefault()
      event.stopImmediatePropagation()
      for (const other of menus()) if (other !== menu) other.open = false
      menu.open = true
      groups.get(menu)?.focus(event.key === 'ArrowUp')
      return
    }
    const current = target?.closest<HTMLDetailsElement>('.tool-menu[open]')
    const parent = current?.parentElement?.closest<HTMLElement>('[data-keyboard-group]')
    if (
      current &&
      !summary &&
      !editingControl(target) &&
      parent?.dataset.keyboardAxis === 'horizontal' &&
      ['ArrowLeft', 'ArrowRight'].includes(event.key)
    ) {
      current.open = false
      current.querySelector<HTMLElement>(':scope > summary')?.focus()
      navigateGroup(parent, event)
      const next = root.activeElement?.closest<HTMLDetailsElement>('.tool-menu')
      if (next) {
        next.open = true
        groups.get(next)?.focus()
      }
      return
    }
    if (event.key !== 'Escape') return
    if (target?.closest('[data-keyboard-scope]') && !target.closest('.tool-menu')) return
    const opened = [...menus()]
    if (!opened.length) return
    for (const menu of opened) menu.open = false
    event.preventDefault()
    event.stopImmediatePropagation()
    opened.at(-1)?.querySelector<HTMLElement>('summary')?.focus()
  }
  const observer = new MutationObserver(setup)
  observer.observe(root, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['open'],
  })
  setup()
  root.addEventListener('pointerdown', dismissOutside, true)
  root.addEventListener('click', dismissOutside, true)
  root.addEventListener('keydown', keyboard, true)
  return () => {
    observer.disconnect()
    root.removeEventListener('pointerdown', dismissOutside, true)
    root.removeEventListener('click', dismissOutside, true)
    root.removeEventListener('keydown', keyboard, true)
  }
}

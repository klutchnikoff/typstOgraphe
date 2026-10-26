/** Dismiss a menu without executing an entry or consuming the outside click. */
export function installMenus(root: Document = document) {
  const menus = () => root.querySelectorAll<HTMLDetailsElement>('.tool-menu[open]')
  const dismissOutside = (event: Event) => {
    const target = event.target
    if (!(target instanceof Element)) return
    const inside = target.closest('.tool-menu')
    for (const menu of menus()) if (menu !== inside) menu.open = false
  }
  const escape = (event: KeyboardEvent) => {
    if (event.isComposing || event.key !== 'Escape') return
    const opened = [...menus()]
    if (!opened.length) return
    for (const menu of opened) menu.open = false
    // Escape belongs to the open menu, not to a panel or the document behind it.
    event.preventDefault()
    event.stopImmediatePropagation()
    opened.at(-1)?.querySelector<HTMLElement>('summary')?.focus()
  }
  root.addEventListener('pointerdown', dismissOutside, true)
  root.addEventListener('click', dismissOutside, true)
  root.addEventListener('keydown', escape, true)
  return () => {
    root.removeEventListener('pointerdown', dismissOutside, true)
    root.removeEventListener('click', dismissOutside, true)
    root.removeEventListener('keydown', escape, true)
  }
}

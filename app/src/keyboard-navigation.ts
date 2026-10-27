/** Shared keyboard contracts. Native fields keep their editing keys. */
export function composing(event: KeyboardEvent) {
  return event.isComposing || event.keyCode === 229
}
function plain(event: KeyboardEvent) {
  return !composing(event) && !event.altKey && !event.ctrlKey && !event.metaKey
}
export function visible(element: HTMLElement) {
  if (!element.isConnected || element.closest('[hidden], [inert], dialog:not([open])')) return false
  let parent = element.closest('details:not([open])')
  while (parent) {
    if (!parent.querySelector(':scope > summary')?.contains(element)) return false
    parent = parent.parentElement?.closest('details:not([open])') || null
  }
  const style = getComputedStyle(element)
  return style.display !== 'none' && style.visibility !== 'hidden'
}
function available(element: HTMLElement) {
  return visible(element) && !element.matches(':disabled, [aria-disabled="true"]')
}
function field(element: EventTarget | null) {
  if (!(element instanceof Element)) return false
  const editable = element.closest('[contenteditable]')
  return (
    !!element.closest('input, textarea, select') ||
    (!!editable && editable.getAttribute('contenteditable') !== 'false')
  )
}

function consume(event: KeyboardEvent) {
  event.preventDefault()
  event.stopPropagation()
}
let nextId = 0
interface NavigationOptions {
  axis: 'vertical' | 'horizontal' | 'both'
  items?: string
  input?: HTMLInputElement
  roving?: boolean
  activateOnMove?: boolean
  initial?: boolean
  enter?: boolean
  beforeNavigate?: () => void
  itemRole?: string
  onSelect?: (item: HTMLElement) => void
  onActivate?: (item: HTMLElement) => void
}
const navigators = new WeakMap<HTMLElement, ReturnType<typeof createNavigation>>()

/** Delegation and live item discovery cover new/removed/disabled choices automatically. */
export function navigationGroup(root: HTMLElement, options: NavigationOptions) {
  navigators.get(root)?.destroy()
  const navigation = createNavigation(root, options)
  navigators.set(root, navigation)
  return navigation
}
/** Used by a nested menu to hand its other axis back to the enclosing toolbar. */
export function navigateGroup(root: HTMLElement, event: KeyboardEvent) {
  navigators.get(root)?.handle(event)
}
function createNavigation(root: HTMLElement, options: NavigationOptions) {
  root.dataset.keyboardGroup = ''
  root.dataset.keyboardAxis = options.axis
  let selected: HTMLElement | undefined
  let lastIndex = 0
  const items = () =>
    [...root.querySelectorAll<HTMLElement>(options.items || 'button, summary, a[href]')].filter(
      (item) => item.closest('[data-keyboard-group]') === root && available(item),
    )
  const allItems = () =>
    [...root.querySelectorAll<HTMLElement>(options.items || 'button, summary, a[href]')].filter(
      (item) => item.closest('[data-keyboard-group]') === root,
    )
  const select = (item?: HTMLElement, reveal = false, notify = false) => {
    selected = item
    const all = allItems()
    for (const choice of all) {
      if (options.input) choice.setAttribute('aria-selected', String(choice === item))
      if (options.input || options.roving)
        choice.tabIndex = choice === item && !options.input ? 0 : -1
    }
    if (options.input) {
      if (item) options.input.setAttribute('aria-activedescendant', item.id)
      else options.input.removeAttribute('aria-activedescendant')
    }
    if (item) {
      lastIndex = all.indexOf(item)
      if (reveal) item.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' })
      if (notify) options.onSelect?.(item)
    }
  }
  const refresh = (reset = false) => {
    const all = allItems(),
      enabled = items()
    if (options.itemRole) for (const item of all) item.setAttribute('role', options.itemRole)
    if (options.input)
      for (const item of all) {
        item.id ||= `keyboard-choice-${++nextId}`
        item.setAttribute('role', 'option')
      }
    const focused = enabled.find((item) => item === document.activeElement)
    const preferred = enabled.find((item) =>
      item.matches(
        '[data-default], [aria-checked="true"], [aria-selected="true"], [aria-pressed="true"]',
      ),
    )
    select(
      focused ||
        (!reset && selected && enabled.includes(selected) ? selected : undefined) ||
        (options.initial === false ? undefined : preferred || enabled[0]),
    )
  }
  const activate = (item: HTMLElement) => {
    if (!available(item)) return
    if (options.onActivate) options.onActivate(item)
    else item.click()
  }
  const handle = (event: KeyboardEvent) => {
    if (event.defaultPrevented || !plain(event) || (event.shiftKey && event.key !== 'Enter')) return
    if (event.target !== options.input && field(event.target)) return
    const arrows =
      options.axis === 'vertical'
        ? ['ArrowUp', 'ArrowDown']
        : options.axis === 'horizontal'
          ? ['ArrowLeft', 'ArrowRight']
          : ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']
    const move = arrows.includes(event.key)
    const boundary = !options.input && ['Home', 'End'].includes(event.key)
    const enter =
      options.input && event.key === 'Enter' && !event.shiftKey && options.enter !== false
    if (!move && !boundary && !enter) return
    consume(event)
    options.beforeNavigate?.()
    const choices = items()
    if (!choices.length) {
      select()
      return
    }
    const current = choices.findIndex(
      (item) => item === document.activeElement || item.contains(document.activeElement),
    )
    const index = current >= 0 ? current : choices.indexOf(selected!)
    if (enter) {
      if (index >= 0) activate(choices[index])
      return
    }
    const backwards = ['ArrowUp', 'ArrowLeft'].includes(event.key)
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? choices.length - 1
          : index < 0
            ? backwards
              ? choices.length - 1
              : 0
            : (index + (backwards ? -1 : 1) + choices.length) % choices.length
    const item = choices[next]
    select(item, true, true)
    if (!options.input) item.focus({ preventScroll: true })
    if (options.activateOnMove) {
      activate(item)
      // Radio/tab rendering may replace all children synchronously.
      if (!item.isConnected) {
        const replacement = items()[next]
        if (replacement) {
          select(replacement)
          replacement.focus({ preventScroll: true })
        }
      }
    }
  }
  const focus = (event: FocusEvent) => {
    const item = items().find((item) => item === event.target)
    if (item) select(item)
  }
  const pointer = (event: MouseEvent) => {
    if (options.input && event.button === 0) event.preventDefault()
  }
  const click = (event: MouseEvent) => {
    if (!options.input) return
    options.beforeNavigate?.()
    const item = items().find((item) => event.target instanceof Node && item.contains(event.target))
    if (!item) return
    select(item, false, true)
    options.input.focus({ preventScroll: true })
    options.onActivate?.(item)
  }
  if (options.input) {
    root.id ||= `keyboard-list-${++nextId}`
    root.setAttribute('role', 'listbox')
    options.input.setAttribute('role', 'combobox')
    options.input.setAttribute('aria-controls', root.id)
    options.input.setAttribute('aria-expanded', 'true')
    options.input.setAttribute('aria-autocomplete', 'list')
    options.input.addEventListener('keydown', handle)
    root.addEventListener('mousedown', pointer)
    root.addEventListener('click', click)
  } else root.addEventListener('keydown', handle)
  root.addEventListener('focusin', focus)
  const observer = new MutationObserver(() => {
    if (!root.isConnected) return
    const lostFocus = selected && !selected.isConnected && document.activeElement === document.body
    const index = lastIndex
    refresh()
    if (lostFocus && !options.input) {
      const next = items()[Math.min(index, items().length - 1)]
      if (next) {
        select(next)
        next.focus({ preventScroll: true })
      } else {
        root.tabIndex = -1
        root.focus({ preventScroll: true })
      }
    }
  })
  observer.observe(root, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['disabled', 'aria-disabled', 'aria-checked', 'aria-pressed', 'data-default'],
  })
  return {
    refresh,
    focus: (last = false) => {
      refresh(true)
      const choices = items(),
        item = last ? choices.at(-1) : choices[0]
      if (item) {
        select(item, true)
        item.focus({ preventScroll: true })
      }
    },
    select: (item?: HTMLElement) => select(item),
    handle,
    fromScope: (event: KeyboardEvent) => handle(event),
    destroy() {
      observer.disconnect()
      root.removeEventListener('keydown', handle)
      root.removeEventListener('focusin', focus)
      root.removeEventListener('mousedown', pointer)
      root.removeEventListener('click', click)
      options.input?.removeEventListener('keydown', handle)
    },
  }
}

export function actionRow() {
  const row = document.createElement('div')
  row.className = 'panel-actions'
  navigationGroup(row, { axis: 'horizontal' })
  return row
}

interface ScopeOptions {
  close?: () => void
  confirm?: () => void
  canConfirm?: () => boolean
  active?: () => boolean
  priority?: number
  tab?: boolean
  enterMultiline?: boolean
  tabExit?: () => void
  initialGroup?: () => HTMLElement | null
}
const scopes = new WeakMap<HTMLElement, ScopeOptions>()
const scopeHandlers = new WeakMap<HTMLElement, (event: KeyboardEvent) => void>()
const installed = new WeakSet<Document>()
/** One Escape resolver per document: nearest scope, then highest visible priority. */
export function keyboardScope(root: HTMLElement, options: ScopeOptions) {
  root.dataset.keyboardScope = ''
  scopes.set(root, options)
  installKeyboardNavigation(root.ownerDocument)
  const previous = scopeHandlers.get(root)
  if (previous) root.removeEventListener('keydown', previous)
  const handle = (event: KeyboardEvent) => {
    if (event.defaultPrevented || !plain(event) || options.active?.() === false) return
    const target = event.target
    if (event.key.startsWith('Arrow') && target === root) {
      const group = options.initialGroup?.()
      if (group) navigators.get(group)?.fromScope(event)
      return
    }
    if (event.key === 'Tab' && options.tab) {
      const controls = [
        ...root.querySelectorAll<HTMLElement>('button, input, textarea, select, [tabindex]'),
      ].filter((element) => element.tabIndex >= 0 && available(element))
      const index = controls.indexOf(document.activeElement as HTMLElement)
      const next = controls[index + (event.shiftKey ? -1 : 1)]
      // Leave the scope at its boundary; the caller may keep an editor bookmark.
      if (!next) {
        if (options.tabExit) {
          consume(event)
          options.tabExit()
        }
        return
      }
      consume(event)
      next.focus()
    } else if (event.key === 'Enter' && !event.shiftKey) {
      if (
        target instanceof HTMLSelectElement ||
        (target instanceof HTMLInputElement && ['file', 'date', 'color'].includes(target.type))
      )
        return
      if (target instanceof HTMLTextAreaElement && !options.enterMultiline) return
      if (target instanceof HTMLButtonElement) {
        consume(event)
        if (available(target)) target.click()
      } else if (options.confirm && options.canConfirm?.() !== false) {
        consume(event)
        options.confirm()
      }
    }
  }
  scopeHandlers.set(root, handle)
  root.addEventListener('keydown', handle)
}
export function installKeyboardNavigation(root: Document = document) {
  if (installed.has(root)) return
  installed.add(root)
  root.addEventListener('keydown', (event) => {
    if (event.defaultPrevented || !plain(event) || event.key !== 'Escape') return
    const open = [...root.querySelectorAll<HTMLElement>('[data-keyboard-scope]')].filter(
      (element) =>
        visible(element) && scopes.get(element)?.close && scopes.get(element)?.active?.() !== false,
    )
    const target = event.target instanceof Element ? event.target : null
    const nearest = target ? open.filter((element) => element.contains(target)).at(-1) : undefined
    const current =
      nearest ||
      open.sort((a, b) => (scopes.get(b)?.priority || 0) - (scopes.get(a)?.priority || 0))[0]
    if (!current) return
    consume(event)
    event.stopImmediatePropagation()
    scopes.get(current)!.close!()
  })
}

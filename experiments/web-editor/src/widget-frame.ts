import type { ViewMutationRecord } from '@tiptap/pm/view'
import { icon, availableIcon } from './icons'
import { tr } from './i18n'

// Settings describe implemented secondary controls, not the Typst parameter signature.
// For example, the quote's attribution is already edited directly in its body.
const widgets = {
  figure: { label: 'Image', settings: true },
  quote: { label: 'Citation', settings: false },
  descriptionList: { label: 'Descriptions', settings: true },
  bulletList: { label: 'Liste à puces', settings: true },
  orderedList: { label: 'Liste numérotée', settings: true },
  postscript: { label: 'P.-S.', settings: false },
  attachment: { label: 'P.J.', settings: false },
  table: { label: 'Tableau', settings: false },
} as const
export type WidgetKind = keyof typeof widgets
export type WidgetAction = (pos: number, kind: WidgetKind, remove?: boolean) => void
export type WidgetIcons = Partial<Record<WidgetKind, string>>
export const isWidgetKind = (kind: string): kind is WidgetKind => Object.hasOwn(widgets, kind)
export const hasWidgetSettings = (kind: string) => isWidgetKind(kind) && widgets[kind].settings

/** The only constructor for block-widget chrome, including maintained NodeViews. */
export function widgetFrame(
  kind: WidgetKind,
  getPos: () => number | undefined,
  action: WidgetAction,
  dom: HTMLElement = document.createElement('section'),
  iconName?: string,
) {
  dom.classList.add('content-widget', `widget-${kind}`)
  dom.dataset.editorWidget = kind
  const slot = document.createElement('div')
  slot.className = 'band-slot'
  slot.contentEditable = 'false'
  const bar = document.createElement('div')
  bar.className = 'widget-bar'
  bar.contentEditable = 'false'
  const title = document.createElement('span')
  const label = tr(widgets[kind].label)
  title.title = label
  title.className = 'widget-kind'
  const glyph = iconName && availableIcon(iconName)
  if (glyph) {
    title.innerHTML = glyph
    title.setAttribute('role', 'img')
    title.setAttribute('aria-label', label)
    title.querySelector('svg')?.setAttribute('aria-hidden', 'true')
  } else title.textContent = label
  bar.append(title)
  for (const [name, iconName, remove] of [
    [tr('Supprimer'), 'trash-2', true],
    [tr('Paramètres'), 'settings-2', false],
  ] as const) {
    const button = document.createElement('button')
    button.type = 'button'
    button.title = `${name} · ${label}`
    button.setAttribute('aria-label', button.title)
    button.disabled = !remove && !hasWidgetSettings(kind)
    if (button.disabled) button.title += ` — ${tr('Aucun paramètre supplémentaire à modifier.')}`
    button.innerHTML = icon(iconName)
    button.onmousedown = (event) => {
      if (event.button === 0) event.preventDefault()
    }
    button.onclick = () => {
      if (button.disabled) return
      const pos = getPos()
      if (pos !== undefined) action(pos, kind, remove)
    }
    bar.append(button)
  }
  dom.prepend(bar, slot)
  return {
    dom,
    content(contentDOM: HTMLElement, delegate?: (record: ViewMutationRecord) => boolean) {
      contentDOM.dataset.widgetBody = ''
      return {
        contentDOM,
        ignoreMutation(record: ViewMutationRecord) {
          if (record.type === 'selection') return false
          if (!contentDOM.contains(record.target)) return true
          return delegate?.(record) ?? false
        },
      }
    },
  }
}

/** Both prose panels and widget-header panels use this marker, never a list of HTML tags. */
export function enclosingWidget(element: Element) {
  return element.closest<HTMLElement>('[data-editor-widget]')
}

import type { ViewMutationRecord } from '@tiptap/pm/view'
import { icon, availableIcon } from './icons'
import { tr } from './i18n'
import type { Capability } from './commands'
import { hasSettings, settingsForNode, type WidgetSettings } from './capability-settings'

const labels = {
  figure: 'Image',
  quote: 'Citation',
  descriptionList: 'Descriptions',
  bulletList: 'Liste à puces',
  orderedList: 'Liste numérotée',
  postscript: 'P.-S.',
  attachment: 'P.J.',
  table: 'Tableau',
} as const
export type WidgetKind = keyof typeof labels
export type WidgetAction = (pos: number, kind: WidgetKind, remove?: boolean) => void
export type WidgetPresentation = { icon: string; settings: WidgetSettings }
export type WidgetPresentations = Partial<Record<WidgetKind, WidgetPresentation>>
export const isWidgetKind = (kind: string): kind is WidgetKind => Object.hasOwn(labels, kind)
export const widgetLabel = (kind: string) => (isWidgetKind(kind) ? tr(labels[kind]) : kind)
export function widgetPresentations(capabilities: Capability[]): WidgetPresentations {
  return Object.fromEntries(
    capabilities
      .filter((entry) => entry.node && isWidgetKind(entry.node))
      .map((entry) => [
        entry.node!,
        { icon: entry.icon, settings: settingsForNode(entry.node!, capabilities) },
      ]),
  )
}

/** The only constructor for block-widget chrome, including maintained NodeViews. */
export function widgetFrame(
  kind: WidgetKind,
  getPos: () => number | undefined,
  action: WidgetAction,
  dom: HTMLElement = document.createElement('section'),
  presentation?: WidgetPresentation,
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
  const label = widgetLabel(kind)
  title.title = label
  title.className = 'widget-kind'
  const glyph = presentation?.icon && availableIcon(presentation.icon)
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
    button.disabled = !remove && (!presentation || !hasSettings(presentation.settings))
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

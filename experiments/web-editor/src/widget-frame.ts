import type { ViewMutationRecord } from '@tiptap/pm/view'
import { icon } from './icons'
import { tr } from './i18n'

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
export const isWidgetKind = (kind: string): kind is WidgetKind => Object.hasOwn(labels, kind)

/** The only constructor for block-widget chrome, including maintained NodeViews. */
export function widgetFrame(
  kind: WidgetKind,
  getPos: () => number | undefined,
  action: WidgetAction,
  dom: HTMLElement = document.createElement('section'),
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
  const label = tr(labels[kind])
  title.textContent = label
  bar.append(title)
  for (const [name, iconName, remove] of [
    [tr('Supprimer'), 'trash-2', true],
    [tr('Paramètres'), 'settings-2', false],
  ] as const) {
    const button = document.createElement('button')
    button.type = 'button'
    button.title = `${name} · ${label}`
    button.setAttribute('aria-label', button.title)
    button.innerHTML = icon(iconName)
    button.onmousedown = (event) => {
      if (event.button === 0) event.preventDefault()
    }
    button.onclick = () => {
      const pos = getPos()
      if (pos !== undefined) action(pos, kind, remove)
    }
    bar.append(button)
  }
  dom.prepend(slot, bar)
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

import { Extension, type Editor } from '@tiptap/core'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import { Plugin, PluginKey, type SelectionBookmark } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { enclosingWidget, isWidgetKind } from './widget-frame'
import type { ContextPanel } from './context-panel'

export type PanelTarget = { from: number; to: number } & {
  placement: 'widget-header' | 'text'
}
interface Anchor {
  from: number
  to: number
  pos: number
  placement: PanelTarget['placement']
  gap: HTMLElement
  bookmark: SelectionBookmark
}
const key = new PluginKey<Anchor | null>('context-band')
export const ContextBand = Extension.create({
  name: 'contextBand',
  addProseMirrorPlugins() {
    return [
      new Plugin<Anchor | null>({
        key,
        state: {
          init: () => null,
          apply(tr, previous) {
            const meta = tr.getMeta(key)
            if (meta !== undefined) return meta
            return previous
              ? {
                  ...previous,
                  from: tr.mapping.map(previous.from, 1),
                  to: tr.mapping.map(previous.to, -1),
                  pos: tr.mapping.map(previous.pos, -1),
                  bookmark: previous.bookmark.map(tr.mapping),
                }
              : null
          },
        },
        props: {
          decorations(state) {
            const anchor = key.getState(state)
            if (!anchor) return null
            if (anchor.placement === 'widget-header') {
              const node = state.doc.nodeAt(anchor.from)
              return node && anchor.from + node.nodeSize <= state.doc.content.size
                ? DecorationSet.create(state.doc, [
                    Decoration.node(anchor.from, anchor.from + node.nodeSize, {
                      class: 'band-open',
                    }),
                  ])
                : null
            }
            return DecorationSet.create(state.doc, [
              Decoration.widget(Math.min(anchor.pos, state.doc.content.size), () => anchor.gap, {
                side: -1,
                ignoreSelection: true,
              }),
            ])
          },
        },
      }),
    ]
  },
})
/** Presentation only: spacing lives in decorations/NodeViews, never in the shared document. */
export class BandHost {
  private panel?: HTMLFormElement
  private shell?: ContextPanel
  private onClose?: () => void
  private frame = 0
  private slot?: HTMLElement
  private observer: ResizeObserver
  private layer: HTMLElement
  private reanchor = true
  constructor(private editor: Editor) {
    this.layer = document.createElement('div')
    this.layer.className = 'band-layer'
    document.querySelector('#editor-scroll')!.append(this.layer)
    this.observer = new ResizeObserver(() => {
      this.reanchor = true
      this.schedule()
    })
    this.observer.observe(document.querySelector('#editor')!)
    editor.on('transaction', ({ transaction }) => {
      if (transaction.docChanged) this.reanchor = true
      this.schedule()
    })
    editor.on('destroy', () => {
      this.observer.disconnect()
      cancelAnimationFrame(this.frame)
      this.layer.remove()
    })
  }
  get isOpen() {
    return !!this.panel
  }
  forPanel(panel: HTMLElement) {
    return this.panel === panel ? this.target || null : null
  }
  get target() {
    return key.getState(this.editor.state)
  }
  open(shell: ContextPanel, target: PanelTarget, onClose?: () => void) {
    const panel = shell.element
    if (this.editor.view.composing) return false
    if (
      !Number.isInteger(target.from) ||
      !Number.isInteger(target.to) ||
      target.from < 0 ||
      target.to < target.from ||
      target.to > this.editor.state.doc.content.size ||
      (target.placement === 'widget-header' &&
        !isWidgetKind(this.editor.state.doc.nodeAt(target.from)?.type.name || ''))
    )
      return false
    this.close(false)
    yUndoPluginKey.getState(this.editor.state)?.undoManager.stopCapturing()
    const gap = document.createElement('span')
    gap.className = 'interline-band-gap'
    gap.contentEditable = 'false'
    gap.setAttribute('aria-hidden', 'true')
    this.panel = panel
    this.shell = shell
    this.onClose = onClose
    panel.hidden = false
    panel.classList.add('context-band')
    this.layer.append(panel)
    this.observer.observe(panel)
    this.editor.view.dispatch(
      this.editor.state.tr.setMeta(key, {
        ...target,
        pos: target.from,
        gap,
        bookmark: this.editor.state.selection.getBookmark(),
      } satisfies Anchor),
    )
    this.reanchor = true
    this.layout()
    this.schedule()
    shell.focus()
    this.reveal()
    return true
  }
  close(focus = true) {
    if (!this.panel) return
    yUndoPluginKey.getState(this.editor.state)?.undoManager.stopCapturing()
    const anchor = this.target
    this.observer.unobserve(this.panel)
    this.panel.hidden = true
    this.panel = undefined
    this.shell = undefined
    if (this.slot) {
      this.slot.style.height = ''
      this.slot = undefined
    }
    this.editor.view.dispatch(this.editor.state.tr.setMeta(key, null))
    this.onClose?.()
    this.onClose = undefined
    if (focus && anchor) {
      this.editor.view.dispatch(
        this.editor.state.tr
          .setSelection(anchor.bookmark.resolve(this.editor.state.doc))
          .scrollIntoView(),
      )
      this.editor.view.focus()
    }
  }
  reveal() {
    this.layout()
    let target: HTMLElement | undefined = this.panel
    const anchor = this.target
    if (anchor?.placement === 'widget-header') {
      const host = this.editor.view.nodeDOM(anchor.from)
      const height = document.querySelector<HTMLElement>('#editor-scroll')!.clientHeight
      if (host instanceof HTMLElement && host.getBoundingClientRect().height <= height)
        target = host
    }
    target?.scrollIntoView({ block: 'nearest' })
  }
  focus() {
    this.shell?.focus()
    this.reveal()
  }
  schedule() {
    if (!this.panel || this.frame) return
    this.frame = requestAnimationFrame(() => {
      this.frame = 0
      this.layout()
    })
  }
  private layout() {
    const anchor = this.target,
      panel = this.panel
    if (!anchor || !panel) return
    const { view } = this.editor
    if (anchor.from < 0 || anchor.to > view.state.doc.content.size) {
      this.close(false)
      return
    }
    const scroll = document.querySelector<HTMLElement>('#editor-scroll')!
    let host: HTMLElement
    let marker: HTMLElement
    if (anchor.placement === 'widget-header') {
      host = view.nodeDOM(anchor.from) as HTMLElement
      if (!(host instanceof HTMLElement)) return
      const slot = host.querySelector<HTMLElement>(':scope > .band-slot')
      if (!slot) return
      this.slot = marker = slot
    } else {
      // Hide only the inert spacer to measure natural wrapping. The focused form stays mounted.
      if (this.reanchor) {
        anchor.gap.style.display = 'none'
        let previousInline = view.coordsAtPos(anchor.from)
        const start = view.state.doc.resolve(anchor.from).start()
        let pos = anchor.from
        while (pos > start && pos > anchor.from - 2000) {
          const previous = view.coordsAtPos(pos - 1)
          // A visual line can mix fonts, sizes and raised inline objects. Their
          // glyph tops differ; adjacent glyph boxes still overlap vertically.
          if (
            Math.min(previous.bottom, previousInline.bottom) -
              Math.max(previous.top, previousInline.top) <=
            0.5
          )
            break
          previousInline = previous
          pos--
        }
        anchor.gap.style.display = ''
        this.reanchor = false
        if (pos !== anchor.pos) {
          view.dispatch(view.state.tr.setMeta(key, { ...anchor, pos }))
          this.layout()
          return
        }
      }
      const dom = view.domAtPos(anchor.from).node
      const element = dom instanceof HTMLElement ? dom : dom.parentElement!
      host = enclosingWidget(element) || view.dom
      marker = anchor.gap
    }
    const hostRect = host.getBoundingClientRect(),
      scrollRect = scroll.getBoundingClientRect()
    const inline = anchor.placement === 'text'
    panel.classList.toggle('inline-context-band', inline)
    const style = getComputedStyle(host)
    const padding = parseFloat(host === view.dom ? style.paddingLeft : style.borderLeftWidth)
    const right = parseFloat(host === view.dom ? style.paddingRight : style.borderRightWidth)
    const width = hostRect.width - padding - right
    panel.style.width = `${width}px`
    const height = panel.getBoundingClientRect().height
    const reserved = height + (inline ? 20 : 0)
    if (Math.abs(parseFloat(marker.style.height || '0') - reserved) > 0.5)
      marker.style.height = `${reserved}px`
    const rect = marker.getBoundingClientRect()
    panel.style.left = `${hostRect.left + padding - scrollRect.left + scroll.scrollLeft}px`
    panel.style.top = `${rect.top - scrollRect.top + scroll.scrollTop + (inline ? 6 : 0)}px`
    if (inline) {
      const start = view.domAtPos(anchor.from, 1),
        end = view.domAtPos(anchor.to, -1)
      const range = document.createRange()
      range.setStart(start.node, start.offset)
      range.setEnd(end.node, end.offset)
      const word =
        [...range.getClientRects()].find((rect) => rect.width > 0 && rect.height > 0) ||
        view.coordsAtPos(anchor.from)
      const x = (word.left + word.right) / 2 - hostRect.left - padding
      panel.style.setProperty('--pointer-x', `${Math.max(12, Math.min(width - 12, x))}px`)
      delete marker.dataset.marker
      const li = marker.closest('li')
      const atStart = anchor.pos === view.state.doc.resolve(anchor.from).start()
      if (li && atStart && li.firstElementChild?.contains(marker)) {
        const list = li.parentElement!
        marker.dataset.marker =
          list.tagName === 'OL'
            ? `${Number(list.getAttribute('start') || 1) + [...list.children].indexOf(li)}.`
            : '•'
      }
    }
  }
}

import type { Editor } from '@tiptap/core'
import type { Transport, Provider } from './provider'
interface Result {
  revision: number
  pages: string[]
  pageSizes?: [number, number][]
}
interface Highlight {
  page: number
  corners: [number, number][]
}
export function previewController(
  editor: Editor,
  provider: Provider,
  transport: Transport,
  version: () => number,
) {
  const container = document.getElementById('pages')!
  let result: Result | undefined,
    shown = -1,
    ticket = 0,
    displayTicket = 0,
    timer: ReturnType<typeof setTimeout>
  const clear = () => {
    for (const svg of container.querySelectorAll('svg')) svg.replaceChildren()
  }
  const selection = () => {
    clearTimeout(timer)
    timer = setTimeout(() => {
      const id = ++ticket
      if (!result || shown !== version()) {
        clear()
        return
      }
      const revision = result.revision,
        { from, to } = editor.state.selection
      void Promise.all([
        transport.invoke<Highlight[]>('sync_selection', { revision, from, to }),
        transport.invoke<{ page: number; point: [number, number] } | null>('sync_forward', {
          revision,
          position: from,
        }),
      ])
        .then(([highlights, point]) => {
          if (id !== ticket || shown !== version() || provider.revision !== revision) return
          clear()
          for (const highlight of highlights) {
            const svg = container.querySelector<SVGSVGElement>(
              `[data-page="${highlight.page}"] svg`,
            )
            if (!svg) continue
            const polygon = document.createElementNS('http://www.w3.org/2000/svg', 'polygon')
            polygon.setAttribute(
              'points',
              highlight.corners.map((pair) => pair.join(',')).join(' '),
            )
            svg.append(polygon)
          }
          if (point && editor.isFocused) {
            const page = container.querySelector<HTMLElement>(`[data-page="${point.page}"]`)
            const size = result!.pageSizes?.[point.page]
            if (!page || !size) return
            const rect = page.getBoundingClientRect(),
              viewport = container.getBoundingClientRect()
            const y = rect.top + (point.point[1] * rect.width) / size[0]
            if (y < viewport.top + 35 || y > viewport.bottom - 35)
              container.scrollTop += y - viewport.top - container.clientHeight / 3
          }
        })
        .catch(() => {})
    }, 35)
  }
  return {
    selection,
    invalidate: () => {
      ticket++
      displayTicket++
      clear()
    },
    show: async (value: Result, currentVersion: number) => {
      const id = ++displayTicket,
        generation = provider.generation
      const current = () =>
        id === displayTicket &&
        currentVersion === version() &&
        value.revision === provider.revision &&
        generation === provider.generation &&
        !editor.isDestroyed
      const images: HTMLImageElement[] = []
      const pages = value.pages.map((src, index) => {
        const page = document.createElement('div')
        page.className = 'preview-page'
        page.dataset.page = String(index)
        const image = document.createElement('img')
        image.src = src
        image.alt = `Page ${index + 1} du rendu Typst`
        images.push(image)
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
        const size = value.pageSizes?.[index]
        if (size) svg.setAttribute('viewBox', `0 0 ${size[0]} ${size[1]}`)
        page.append(image, svg)
        const displayed = () =>
          result === value && shown === version() && provider.revision === value.revision
        page.ondblclick = (event) => {
          if (!displayed() || !size) return
          const rect = image.getBoundingClientRect()
          const location = {
            page: index,
            point: [
              ((event.clientX - rect.left) * size[0]) / rect.width,
              ((event.clientY - rect.top) * size[1]) / rect.height,
            ],
          }
          void transport
            .invoke<number | null>('sync_backward', { revision: value.revision, location })
            .then((pos) => {
              if (pos !== null && displayed())
                editor.chain().focus().setTextSelection(pos).scrollIntoView().run()
            })
            .catch(() => {})
        }
        return page
      })
      try {
        // Decode detached images before replacing any visible page. Publishing
        // all pages together also keeps layout and sync on the same revision.
        await Promise.all(images.map((image) => image.decode()))
      } catch (error) {
        if (!current()) return false
        throw error
      }
      if (!current()) return false
      ticket++
      result = value
      shown = currentVersion
      const scrollTop = container.scrollTop
      container.replaceChildren(...pages)
      container.scrollTop = scrollTop
      selection()
      return true
    },
  }
}

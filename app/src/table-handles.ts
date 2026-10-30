import { Extension, type Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { icon } from './icons'
import { tr } from './i18n'
import { navigationGroup } from './keyboard-navigation'

export type TableAxis = 'row' | 'column'
export interface TableHandleTarget {
  pos: number
  node: PMNode
  row: number
  column: number
}
interface Controls {
  available: (target: TableHandleTarget, axis: TableAxis) => boolean
  open: (target: TableHandleTarget, axis: TableAxis) => void
  append: (target: TableHandleTarget, axis: TableAxis) => void
}
type SelectedAxis = { pos: number; node: PMNode; axis: TableAxis; index: number }
const key = new PluginKey<SelectedAxis | null>('table-handles')
const bindings = new WeakMap<Editor, Controls>()
export function bindTableHandles(editor: Editor, controls: Controls) {
  bindings.set(editor, controls)
  refreshTableHandles(editor)
  editor.on('destroy', () => bindings.delete(editor))
}
export function refreshTableHandles(editor: Editor) {
  if (!editor.isDestroyed) editor.view.dispatch(editor.state.tr.setMeta(key, { refresh: true }))
}
export function selectTableAxis(editor: Editor, selected: SelectedAxis | null) {
  if (!editor.isDestroyed) editor.view.dispatch(editor.state.tr.setMeta(key, { selected }))
}
/** Resolve at activation time. A decorated button never keeps a stale row/path
 * after another participant inserts content before it. */
function targetAt(doc: PMNode, pos: number): TableHandleTarget | undefined {
  const $pos = doc.resolve(pos)
  let figure = 0,
    row = 0,
    column = 0
  for (let depth = 1; depth <= $pos.depth; depth++) {
    const kind = $pos.node(depth).type.name
    if (kind === 'tableFigure') figure = depth
    if (kind === 'tableRow') row = $pos.index(depth - 1)
    if (kind === 'tableCell' || kind === 'tableHeader') column = $pos.index(depth - 1)
  }
  return figure ? { pos: $pos.before(figure), node: $pos.node(figure), row, column } : undefined
}

/** Noneditable decorations, never extra rows/cells or content in Yjs/Typst. */
export const TableHandles = Extension.create({
  name: 'tableHandles',
  addProseMirrorPlugins() {
    const editor = this.editor
    return [
      new Plugin<SelectedAxis | null>({
        key,
        state: {
          init: () => null,
          apply(transaction, previous) {
            const meta = transaction.getMeta(key)
            if (meta && 'selected' in meta) return meta.selected
            if (!previous || !transaction.docChanged) return previous
            const pos = transaction.mapping.map(previous.pos, 1)
            return transaction.doc.nodeAt(pos)?.eq(previous.node) ? { ...previous, pos } : null
          },
        },
        props: {
          decorations(state) {
            const selected = key.getState(state)
            const decorations: Decoration[] = []
            state.doc.descendants((figure, pos) => {
              if (figure.type.name !== 'tableFigure') return
              const grid = figure.firstChild!
              const widget = (
                at: number,
                axis: TableAxis,
                row: number,
                column: number,
                append = false,
              ) => {
                const target = { pos, node: figure, row, column }
                const active =
                  !append &&
                  selected?.pos === pos &&
                  selected.axis === axis &&
                  selected.index === (axis === 'row' ? row : column)
                const enabled = bindings.get(editor)?.available(target, axis) === true
                const label = append
                  ? tr(
                      axis === 'row'
                        ? 'Ajouter une ligne en bas du tableau'
                        : 'Ajouter une colonne à droite du tableau',
                    )
                  : `${tr(axis === 'row' ? 'Actions sur la ligne' : 'Actions sur la colonne')} ${(axis === 'row' ? row : column) + 1}`
                decorations.push(
                  Decoration.widget(
                    at,
                    (view, getPos) => {
                      const button = document.createElement('button')
                      button.type = 'button'
                      button.contentEditable = 'false'
                      button.className = `table-handle ${append ? `table-append-${axis}` : `table-axis-handle table-${axis}-handle`}`
                      button.disabled = !enabled
                      button.title = label
                      button.setAttribute('aria-label', label)
                      if (append) button.dataset.tableAppend = axis
                      else {
                        button.dataset.tableAxis = axis
                        button.dataset.tableIndex = String(axis === 'row' ? row : column)
                        button.setAttribute('aria-expanded', String(!!active))
                        button.setAttribute('aria-pressed', String(!!active))
                      }
                      button.innerHTML = append
                        ? icon('plus')
                        : '<span class="table-grip" aria-hidden="true"></span>'
                      button.onmousedown = (event) => {
                        if (event.button === 0) event.preventDefault()
                      }
                      button.onclick = () => {
                        const current = getPos()
                        const target =
                          current === undefined ? undefined : targetAt(view.state.doc, current)
                        const controls = bindings.get(editor)
                        if (!target || !controls?.available(target, axis)) return
                        if (append) controls.append(target, axis)
                        else controls.open(target, axis)
                      }
                      if (append && axis === 'row') {
                        const row = document.createElement('div')
                        row.contentEditable = 'false'
                        row.className = 'table-append-row-slot'
                        row.append(button)
                        return row
                      }
                      return button
                    },
                    {
                      key: `table-${pos}-${axis}-${row}-${column}-${append}-${enabled}-${!!active}`,
                      side: -1,
                      ignoreSelection: true,
                      stopEvent: () => true,
                    },
                  ),
                )
              }
              let rowPos = pos + 2
              grid.forEach((row, _, r) => {
                let cellPos = rowPos + 1
                row.forEach((cell, _, c) => {
                  if (!c) widget(cellPos + 1, 'row', r, c)
                  if (!r) widget(cellPos + 1, 'column', r, c)
                  if (!r && c === row.childCount - 1) widget(cellPos + 1, 'column', r, c, true)
                  if (
                    selected?.pos === pos &&
                    (selected.axis === 'row' ? selected.index === r : selected.index === c)
                  )
                    decorations.push(
                      Decoration.node(cellPos, cellPos + cell.nodeSize, {
                        class: 'table-axis-selected',
                      }),
                    )
                  cellPos += cell.nodeSize
                })
                rowPos += row.nodeSize
              })
              widget(pos + 1 + grid.nodeSize, 'row', grid.childCount - 1, 0, true)
              return false
            })
            return DecorationSet.create(state.doc, decorations)
          },
        },
        view(view) {
          const groups = new Map<HTMLElement, ReturnType<typeof navigationGroup>>()
          const setup = () => {
            for (const [element, group] of groups)
              if (!view.dom.contains(element)) {
                group.destroy()
                groups.delete(element)
              }
            for (const element of view.dom.querySelectorAll<HTMLElement>(
              '[data-editor-widget="tableFigure"]',
            )) {
              if (groups.has(element)) continue
              groups.set(
                element,
                navigationGroup(element, {
                  axis: 'both',
                  items: () => {
                    const axis =
                      (document.activeElement as HTMLElement | null)?.dataset.tableAxis || 'column'
                    return [
                      ...element.querySelectorAll<HTMLElement>(
                        `.table-axis-handle[data-table-axis="${axis}"]`,
                      ),
                    ]
                  },
                }),
              )
            }
          }
          setup()
          return {
            update: setup,
            destroy: () => {
              for (const group of groups.values()) group.destroy()
            },
          }
        },
      }),
    ]
  },
})

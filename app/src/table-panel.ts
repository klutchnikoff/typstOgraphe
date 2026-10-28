import { tableCursor } from './table-selection'
import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { contextPanel } from './context-panel'
import { navigationGroup } from './keyboard-navigation'
import type { BandHost } from './band-layout'
import type { Provider, Transport } from './provider'
import type { Registry, Capability, TableAction } from './commands'
import { positionAtPath } from './commands'
import { widgetTarget, widgetContinuation } from './widgets'
import { executeStructured, type StructuredCommand } from './structured-commands'
import { tr } from './i18n'
import { nodePath } from './editor-targets'

export function tableContext(editor: Editor) {
  const target = widgetTarget(editor)
  if (target?.node.type.name !== 'tableFigure') return
  const $from = tableCursor(editor)
  let row = 0,
    column = 0
  for (let depth = 1; depth <= $from.depth; depth++) {
    if ($from.node(depth).type.name === 'tableRow') row = $from.index(depth - 1)
    if (['tableCell', 'tableHeader'].includes($from.node(depth).type.name))
      column = $from.index(depth - 1)
  }
  return { ...target, row, column }
}
function tree(path: number[]) {
  return { root: { kind: 'body' }, path }
}
export function tablePanel(
  editor: Editor,
  bands: BandHost,
  provider: Provider,
  transport: Transport,
  capability: Capability,
) {
  const operations = capability.editor?.actions || []
  const rowSpec = capability.parameters.properties!.rows
  const columnSpec = capability.parameters.properties!.columns
  let opened:
    { pos: number; node?: PMNode; row: number; column: number; action?: string } | undefined
  let pending = false
  const panel = contextPanel({
    id: 'table-properties',
    closeLabel: tr('Annuler'),
    close: () => {
      if (!pending) bands.close()
    },
    canConfirm: () => !!opened && !pending && !editor.view.composing,
    tabExit: () => editor.view.focus(),
  })
  const dimensions = document.createElement('div')
  dimensions.className = 'table-dimensions'
  const inputs = ['Lignes', 'Colonnes'].map((label, index) => {
    const field = document.createElement('label')
    field.textContent = tr(label)
    const input = document.createElement('input')
    input.type = 'number'
    const spec = index ? columnSpec : rowSpec
    input.min = String(spec.minimum)
    input.max = String(spec.maximum)
    input.value = String(spec.default)
    input.setAttribute('aria-label', tr(label))
    field.append(input)
    dimensions.append(field)
    return input
  })
  const headerLabel = document.createElement('label')
  headerLabel.className = 'table-header-choice'
  const header = document.createElement('input')
  header.type = 'checkbox'
  headerLabel.append(header, document.createTextNode(tr('Première ligne d’en-tête')))
  const info = document.createElement('p')
  info.className = 'band-hint'
  const choices = document.createElement('div')
  choices.className = 'table-actions'
  choices.role = 'radiogroup'
  choices.setAttribute('aria-label', tr('Modifier le tableau'))
  navigationGroup(choices, { axis: 'both', roving: true, activateOnMove: true })
  const buttons = operations.map(({ id, label, remove }) => {
    const button = document.createElement('button')
    button.type = 'button'
    button.role = 'radio'
    button.textContent = label
    button.setAttribute('aria-label', label)
    button.classList.toggle('danger', remove)
    button.onclick = () => {
      if (!opened || pending) return
      opened.action = id
      renderChoices()
    }
    choices.append(button)
    return button
  })
  panel.body.append(dimensions, headerLabel, info, choices)
  panel.action({
    label: tr('Annuler'),
    run: () => {
      if (!pending) bands.close()
    },
  })
  const apply = panel.action({ label: tr('Valider'), primary: true, run: () => panel.submit() })
  function available(operation: TableAction, node: PMNode) {
    const grid = node.firstChild!
    const row = operation.axis === 'row'
    const count = row ? grid.childCount : grid.firstChild!.childCount
    return operation.remove ? count > 1 : count < Number((row ? rowSpec : columnSpec).maximum)
  }
  function renderChoices() {
    if (!opened?.node) return
    operations.forEach((operation, index) => {
      const { id } = operation
      buttons[index].disabled = pending || !available(operation, opened!.node!)
      buttons[index].setAttribute('aria-checked', String(opened!.action === id))
      buttons[index].tabIndex = opened!.action === id || (!opened!.action && index === 0) ? 0 : -1
    })
  }
  panel.confirm = () => {
    void confirm()
  }
  async function confirm() {
    if (!opened || pending) return
    const draft = opened,
      anchor = bands.target
    if (!anchor || (draft.node && !editor.state.doc.nodeAt(anchor.from)?.eq(draft.node))) {
      panel.error.textContent = tr(
        'Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.',
      )
      return
    }
    const commands: StructuredCommand[] = []
    let path: number[],
      row = draft.row,
      column = draft.column
    if (!draft.node) {
      const rows = Number(inputs[0].value),
        columns = Number(inputs[1].value)
      if (
        !Number.isInteger(rows) ||
        !Number.isInteger(columns) ||
        rows < 1 ||
        rows > Number(rowSpec.maximum) ||
        columns < 1 ||
        columns > Number(columnSpec.maximum)
      ) {
        panel.error.textContent = tr('Dimensions de tableau invalides.')
        return
      }
      const selection = editor.state.doc.resolve(anchor.from)
      if (selection.depth !== 1 || !['paragraph', 'heading'].includes(selection.parent.type.name))
        return
      const index = selection.index(0) + Number(selection.parentOffset > 0)
      if (selection.parentOffset > 0)
        commands.push({
          id: 'block.split',
          params: {
            target: {
              zone: { kind: 'body', path: [selection.index(0)] },
              from: selection.parentOffset,
              to: selection.parentOffset,
            },
          },
        })
      commands.push({
        id: 'insert.table',
        params: { parent: tree([]), index, rows, columns, header: header.checked },
      })
      const continuation = widgetContinuation(
        selection.parent.copy(selection.parent.content.cut(selection.parentOffset)),
      )
      if (continuation)
        commands.push({
          id: 'block.insert',
          params: { parent: tree([]), index: index + 1, node: continuation },
        })
      path = [index]
    } else {
      path = nodePath(editor, anchor.from)
      const target = tree(path)
      const operation = operations.find(({ id }) => id === draft.action)
      if (operation) {
        const { command: id, offset } = operation,
          isRow = operation.axis === 'row'
        commands.push({ id, params: { target, index: (isRow ? row : column) + offset } })
        if (id.endsWith('insert')) {
          if (isRow) row += offset
          else column += offset
        }
      }
      commands.push({ id: 'table.header', params: { target, enabled: header.checked } })
    }
    pending = true
    apply.disabled = true
    renderChoices()
    // Keep the pending band focused until the owner has committed the guarded batch.
    try {
      await executeStructured(editor, provider, transport, commands)
      bands.close()
      const figure = editor.state.doc.nodeAt(positionAtPath(editor, path))
      if (figure?.type.name === 'tableFigure' && (!draft.node || draft.action)) {
        const grid = figure.firstChild!
        row = Math.min(row, grid.childCount - 1)
        column = Math.min(column, grid.firstChild!.childCount - 1)
        editor.commands.setTextSelection(positionAtPath(editor, [...path, 0, row, column, 0]) + 1)
      }
      editor.view.focus()
    } catch (error) {
      bands.close(false)
      provider.onError(error)
    } finally {
      pending = false
      apply.disabled = false
    }
  }
  function open(pos?: number, action?: string) {
    if (pending || editor.view.composing) return
    const context = pos === undefined ? undefined : tableContext(editor)
    const node = pos === undefined ? undefined : editor.state.doc.nodeAt(pos)
    if (pos !== undefined && node?.type.name !== 'tableFigure') return
    const selection = editor.state.selection
    if (
      pos === undefined &&
      (!selection.empty ||
        selection.$from.depth !== 1 ||
        !['paragraph', 'heading'].includes(selection.$from.parent.type.name))
    )
      return
    bands.close(false)
    opened = {
      pos: pos ?? selection.from,
      node: node || undefined,
      row: context?.row || 0,
      column: context?.column || 0,
      action,
    }
    dimensions.hidden = !!node
    choices.hidden = !node
    info.hidden = !node
    panel.title.textContent = tr(node ? 'Modifier le tableau' : 'Insérer un tableau')
    header.checked = node
      ? node.firstChild!.firstChild!.content.content.every(
          (cell) => cell.type.name === 'tableHeader',
        )
      : true
    inputs[0].value = String(rowSpec.default)
    inputs[1].value = String(columnSpec.default)
    info.textContent = node
      ? `${tr('Ligne')} ${opened.row + 1} · ${tr('Colonne')} ${opened.column + 1}`
      : ''
    panel.error.textContent = ''
    renderChoices()
    bands.open(
      panel,
      {
        from: pos ?? selection.from,
        to: pos === undefined ? selection.to : pos + node!.nodeSize,
        placement: node ? 'widget-header' : 'text',
      },
      () => {
        opened = undefined
      },
    )
  }
  return {
    open,
    register(registry: Registry) {
      for (const operation of operations) {
        const { id, label } = operation
        registry.set(id, {
          id,
          label: tr(label),
          enabled: () => {
            const target = tableContext(editor)
            return (
              !pending &&
              provider.online &&
              !editor.view.composing &&
              !!target &&
              available(operation, target.node)
            )
          },
          run: () => {
            const target = tableContext(editor)
            if (target) open(target.pos, id)
          },
        })
      }
    },
  }
}

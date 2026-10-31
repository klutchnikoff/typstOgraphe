import { focusEditorSelection } from './editor-focus'
import { editingAvailability } from './command-availability'
import { tableCursor } from './table-selection'
import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { contextPanel } from './context-panel'
import type { BandHost } from './band-layout'
import type { Provider, Transport } from './provider'
import type { Registry, Capability, TableAction } from './commands'
import { positionAtPath } from './commands'
import { widgetTarget, widgetContinuation } from './widgets'
import { applyPlannedEdit, executeStructured, type StructuredCommand } from './structured-commands'
import { TextSelection } from '@tiptap/pm/state'
import { tr } from './i18n'
import { nodePath } from './editor-targets'
import {
  bindTableHandles,
  refreshTableHandles,
  selectTableAxis,
  type TableAxis,
  type TableHandleTarget,
} from './table-handles'
import { commandCapabilities } from './command-context'

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
    | {
        pos: number
        node?: PMNode
        row: number
        column: number
        action?: string
        axis?: TableAxis
        returnHandle?: boolean
      }
    | undefined
  let pending = false
  let availability = ''
  function refresh() {
    const value = `${pending}:${editor.isEditable}:${provider.online}:${!!provider.rejected}:${editor.view.composing}:${provider.composing}:${commandCapabilities(editor)?.available()?.includes('insert.table')}`
    if (value !== availability) {
      availability = value
      refreshTableHandles(editor)
    }
  }
  function cancel() {
    if (pending) return
    const draft = opened,
      anchor = bands.forPanel(panel.element)
    const pos = anchor?.from
    bands.close()
    if (draft?.returnHandle && pos !== undefined && editor.state.doc.nodeAt(pos)?.eq(draft.node!)) {
      const index = draft.axis === 'row' ? draft.row : draft.column
      const element = editor.view.nodeDOM(pos)
      if (element instanceof HTMLElement)
        element
          .querySelector<HTMLElement>(
            `[data-table-axis="${draft.axis}"][data-table-index="${index}"]`,
          )
          ?.focus()
    }
  }
  const panel = contextPanel({
    id: 'table-properties',
    closeLabel: tr('Annuler'),
    close: cancel,
    canConfirm: () => !!opened && !pending && !editor.view.composing,
    tabExit: () => focusEditorSelection(editor),
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
  const headerOptions = document.createElement('div')
  const headerInputs = ['header', 'row_headers'].map((name) => {
    const label = document.createElement('label')
    label.className = 'table-header-choice'
    const input = document.createElement('input')
    input.type = 'checkbox'
    label.append(
      input,
      document.createTextNode(capability.parameters.properties![name].title || name),
    )
    headerOptions.append(label)
    return input
  })
  const [header, rowHeaders] = headerInputs
  const info = document.createElement('p')
  info.className = 'band-hint'
  panel.body.append(dimensions, headerOptions, info)
  const remove = panel.action({
    label: tr('Supprimer'),
    danger: true,
    run: () => chooseAxisAction(true),
  })
  const dismiss = panel.action({
    label: tr('Annuler'),
    run: cancel,
  })
  const before = panel.action({ label: '', run: () => chooseAxisAction(false, false) })
  const apply = panel.action({ label: tr('Valider'), primary: true, run: () => panel.submit() })
  function available(operation: TableAction, node: PMNode) {
    const grid = node.firstChild!
    const row = operation.axis === 'row'
    const count = row ? grid.childCount : grid.firstChild!.childCount
    return operation.remove ? count > 1 : count < Number((row ? rowSpec : columnSpec).maximum)
  }
  function refreshActions() {
    apply.disabled = before.disabled = remove.disabled = pending
    if (!opened?.node) return
    if (opened.axis) {
      for (const [button, operation] of [
        [remove, axisAction(true)],
        [before, axisAction(false, false)],
        [apply, axisAction(false, true)],
      ] as const)
        button.disabled = pending || !operation || !available(operation, opened.node)
    }
  }

  panel.confirm = () => {
    if (opened?.axis) {
      chooseAxisAction(false, true)
      return
    }
    void confirm()
  }
  function axisAction(remove: boolean, after = true) {
    return operations.find(
      (op) =>
        op.axis === opened?.axis && op.remove === remove && (remove || op.offset === Number(after)),
    )
  }
  function chooseAxisAction(remove: boolean, after = true) {
    if (!opened?.axis || pending) return
    const operation = axisAction(remove, after)
    if (!operation || !opened.node || !available(operation, opened.node)) return
    opened.action = operation.id
    void confirm()
  }
  async function confirm() {
    if (!opened || pending) return
    const draft = opened,
      anchor = bands.forPanel(panel.element)
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
        params: {
          parent: tree([]),
          index,
          rows,
          columns,
          header: header.checked,
          row_headers: rowHeaders.checked,
        },
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
          if (isRow) {
            row += offset
            column = 0
          } else {
            column += offset
          }
        }
      }
      if (!draft.axis) {
        commands.push({
          id: 'table.header',
          params: { target, axis: 'columns', enabled: header.checked },
        })
        commands.push({
          id: 'table.header',
          params: { target, axis: 'rows', enabled: rowHeaders.checked },
        })
      }
    }
    pending = true
    refresh()
    apply.disabled = true
    before.disabled = true
    refreshActions()
    // Keep the pending band focused until the owner has committed the guarded batch.
    try {
      const current = () => opened === draft && !!bands.forPanel(panel.element)
      if (draft.node && !draft.action) {
        // Header roles preserve projected positions even when Yrs replaces a
        // cell's identity. Use the same selection restoration as other settings.
        await applyPlannedEdit(editor, editor.state.tr, commands, { focus: false, current })
      } else {
        await executeStructured(editor, provider, transport, commands, current)
      }
      if (!current()) return
      let returned = editor.state.selection.getBookmark()
      const figure = editor.state.doc.nodeAt(positionAtPath(editor, path))
      if (figure?.type.name === 'tableFigure' && (!draft.node || draft.action)) {
        const grid = figure.firstChild!
        row = Math.min(row, grid.childCount - 1)
        column = Math.min(column, grid.firstChild!.childCount - 1)
        returned = TextSelection.create(
          editor.state.doc,
          positionAtPath(editor, [...path, 0, row, column, 0]) + 1,
        ).getBookmark()
      }
      panel.committed()
      bands.close(true, returned)
    } catch (error) {
      if (bands.forPanel(panel.element)) bands.close(false)
      provider.onError(error)
    } finally {
      pending = false
      selectTableAxis(editor, null)
      refresh()
      refreshActions()
    }
  }
  function open(pos?: number) {
    if (pending || !editingAvailability(editor).enabled) return
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
    }
    dimensions.hidden = !!node
    remove.hidden = true
    before.hidden = true
    headerOptions.hidden = false
    apply.textContent = tr('Valider')
    info.hidden = true
    panel.title.textContent = tr(node ? 'Modifier le tableau' : 'Insérer un tableau')
    const grid = node?.firstChild
    header.checked = grid
      ? grid.firstChild!.content.content.every(
          (cell) =>
            cell.type.name === 'tableHeader' &&
            ['column', 'both'].includes(cell.attrs.scope || 'column'),
        )
      : capability.parameters.properties!.header.default === true
    rowHeaders.checked = grid
      ? grid.content.content.every(
          (row) =>
            row.firstChild!.type.name === 'tableHeader' &&
            ['row', 'both'].includes(row.firstChild!.attrs.scope),
        )
      : capability.parameters.properties!.row_headers.default === true
    inputs[0].value = String(rowSpec.default)
    inputs[1].value = String(columnSpec.default)
    info.textContent = node
      ? `${tr('Ligne')} ${opened.row + 1} · ${tr('Colonne')} ${opened.column + 1}`
      : ''
    panel.error.textContent = ''
    refreshActions()
    bands.open(
      panel,
      {
        from: pos ?? selection.from,
        to: pos === undefined ? selection.to : pos + node!.nodeSize,
        placement: node ? 'widget-header' : 'text',
      },
      () => {
        opened = undefined
        selectTableAxis(editor, null)
      },
    )
  }
  function canOpen(target: TableHandleTarget, axis: TableAxis) {
    return (
      !pending &&
      provider.online &&
      editingAvailability(editor, false).enabled &&
      commandCapabilities(editor)?.available()?.includes('insert.table') !== false &&
      operations.some((op) => op.axis === axis && available(op, target.node))
    )
  }
  function openAxis(target: TableHandleTarget, axis: TableAxis, initial?: TableAction) {
    if (!canOpen(target, axis)) return
    const context = tableContext(editor)
    bands.close(false)
    opened = {
      ...target,
      // Column handles live above the first row, but editing continues on the
      // user's row when its caret belongs to this table.
      row: axis === 'column' && context?.pos === target.pos ? context.row : target.row,
      axis,
      returnHandle: true,
    }
    const draft = opened
    dimensions.hidden = headerOptions.hidden = true
    info.hidden = before.hidden = remove.hidden = false
    info.textContent = `${tr(axis === 'row' ? 'Ligne' : 'Colonne')} ${(axis === 'row' ? target.row : target.column) + 1}`
    panel.title.textContent = tr('Modifier le tableau')
    before.textContent = tr(axis === 'row' ? 'Au-dessus' : 'À gauche')
    apply.textContent = tr(axis === 'row' ? 'Au-dessous' : 'À droite')
    panel.error.textContent = ''
    refreshActions()
    bands.open(
      panel,
      { from: target.pos, to: target.pos + target.node.nodeSize, placement: 'widget-header' },
      () => {
        if (opened === draft) opened = undefined
        selectTableAxis(editor, null)
      },
    )
    selectTableAxis(editor, {
      pos: target.pos,
      node: target.node,
      axis,
      index: axis === 'row' ? target.row : target.column,
    })
    const focus = initial?.remove ? remove : initial?.offset === 0 ? before : apply
    ;(focus.disabled ? dismiss : focus).focus({ preventScroll: true })
  }
  bindTableHandles(editor, {
    available: canOpen,
    open: openAxis,
  })
  panel.recoveryData = () =>
    opened
      ? { new: !opened.node, axis: opened.axis || null, row: opened.row, column: opened.column }
      : null
  return {
    resume(pos: number, data: any) {
      const node = editor.state.doc.nodeAt(pos)
      if (!node || node.type.name !== 'tableFigure') return false
      if (data?.axis === 'row' || data?.axis === 'column') {
        if (
          !Number.isInteger(data.row) ||
          !Number.isInteger(data.column) ||
          data.row < 0 ||
          data.column < 0 ||
          data.row >= node.firstChild!.childCount ||
          data.column >= node.firstChild!.firstChild!.childCount
        )
          return false
        openAxis({ pos, node, row: data.row, column: data.column }, data.axis)
      } else open(pos)
      return !!bands.forPanel(panel.element)
    },
    open,
    refresh,
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
            if (target) openAxis(target, operation.axis, operation)
          },
        })
      }
    },
  }
}

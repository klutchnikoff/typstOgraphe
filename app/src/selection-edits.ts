import { Extension, type Editor } from '@tiptap/core'
import { Fragment, Slice, type Node as PMNode } from '@tiptap/pm/model'
import { Plugin, Selection, TextSelection } from '@tiptap/pm/state'
import type { Step } from '@tiptap/pm/transform'
import { CellSelection } from '@tiptap/pm/tables'
import { clipboardInline } from './clipboard-content'
import { cellPastePlan } from './cell-paste'
import { editorRoot, nodePath } from './editor-targets'
import { isWidgetKind } from './widget-frame'
import { normalizeWhitespace } from './document-whitespace'
import { documentContent } from './shared-inline-marks'
import { dispatchImmediateEdit } from './immediate-edit'
import type { StructuredCommand } from './structured-commands'

interface Range {
  from: number
  to: number
  pos: number
  node: PMNode
  whole?: boolean
}
/** Paragraphs of one prose flow may join. Semantic zones of a widget may not. */
function ranges(editor: Editor, from: number, to: number) {
  const selected: Range[] = []
  if (editor.state.selection instanceof CellSelection) {
    editor.state.selection.forEachCell((cell, pos) => {
      cell.descendants((node, offset) => {
        if (!node.isTextblock) return
        selected.push({
          from: pos + offset + 2,
          to: pos + offset + 2 + node.content.size,
          pos: pos + offset + 1,
          node,
        })
        return false
      })
    })
    return { selected, protected: true }
  }
  const owners = new Set<number>()
  editor.state.doc.nodesBetween(from, to, (node, pos) => {
    if (isWidgetKind(node.type.name) && from <= pos && to >= pos + node.nodeSize) {
      selected.push({ from: pos, to: pos + node.nodeSize, pos, node, whole: true })
      return false
    }
    if (!node.isTextblock) return
    const $pos = editor.state.doc.resolve(pos + 1)
    let owner = -1
    for (let depth = $pos.depth; depth > 0; depth--) {
      if (!['paragraph', 'heading'].includes($pos.node(depth).type.name)) {
        owner = $pos.before(depth)
        break
      }
    }
    owners.add(owner)
    selected.push({
      from: Math.max(from, pos + 1),
      to: Math.min(to, pos + 1 + node.content.size),
      pos,
      node,
    })
    return false
  })
  return { selected, protected: owners.size > 1 }
}
function marks(node: PMNode) {
  return node.marks.map((mark) =>
    mark.type.name === 'link' ? { type: 'link', url: mark.attrs.href } : { type: mark.type.name },
  )
}
/** One transaction and one typed equivalent: only selected contents are removed,
 * except whole widgets explicitly included in the selection. */
export function protectedSelectionPlan(
  editor: Editor,
  slice = Slice.empty,
  range: { from: number; to: number } = editor.state.selection,
) {
  if (range.from === range.to) return
  const selection = ranges(editor, range.from, range.to)
  if (!selection.protected) return
  const first = selection.selected.find((part) => !part.whole)
  if (!first) return
  const root = editorRoot(editor)
  const tr = editor.state.tr
  const commands: StructuredCommand[] = []
  for (const part of [...selection.selected].sort((a, b) => b.from - a.from)) {
    if (part.from >= part.to) continue
    if (root)
      commands.push(
        part.whole
          ? {
              id: 'block.remove',
              params: { target: { root, path: nodePath(editor, part.pos) } },
            }
          : {
              id: 'text.delete',
              params: {
                target: {
                  zone: { ...root, path: nodePath(editor, part.pos) },
                  from: part.from - part.pos - 1,
                  to: part.to - part.pos - 1,
                },
              },
            },
      )
    tr.delete(part.from, part.to)
  }
  const at = tr.mapping.map(first.from, -1)
  const $at = tr.doc.resolve(at)
  const path = Array.from({ length: $at.depth }, (_, depth) => $at.index(depth))
  const nodes = clipboardInline(slice, editor)
  let offset = $at.parentOffset
  for (const node of nodes) {
    if (root)
      commands.push(
        node.isText
          ? {
              id: 'text.insert',
              params: {
                target: { zone: { ...root, path }, from: offset, to: offset },
                text: node.text,
                marks: marks(node),
              },
            }
          : {
              id: 'inline.replace',
              params: {
                target: { zone: { ...root, path }, from: offset, to: offset },
                node: documentContent(node.toJSON()),
              },
            },
      )
    offset += node.nodeSize
  }
  tr.insert(at, Fragment.fromArray(nodes))
  tr.setSelection(
    TextSelection.create(tr.doc, at + nodes.reduce((size, node) => size + node.nodeSize, 0)),
  )
  if (editor.state.storedMarks !== null) tr.setStoredMarks(editor.state.storedMarks)
  const whitespace = editor.extensionManager.extensions.find(
    (e) => e.name === 'documentWhitespace',
  )!
  normalizeWhitespace(tr, whitespace.options.trailingParagraph, whitespace.options.paragraphs)
  return { transaction: tr, commands }
}
function textSlice(editor: Editor, text: string) {
  const marks = editor.state.storedMarks ?? editor.state.selection.$from.marks()
  return new Slice(
    Fragment.fromArray(
      text
        .split(/\r\n?|\n/)
        .flatMap((line, index) => [
          ...(index ? [editor.schema.nodes.softBreak.create()] : []),
          ...(line ? [editor.schema.text(line, marks)] : []),
        ]),
    ),
    0,
    0,
  )
}
export function selectionKey(editor: Editor, event: KeyboardEvent) {
  if (event.isComposing || editor.view.composing || !editor.isEditable) return false
  if (editor.state.selection.empty) return false
  if (!['Backspace', 'Delete', 'Enter'].includes(event.key)) return false
  if (event.key === 'Enter' && (event.ctrlKey || event.metaKey || event.altKey)) return false
  const slice =
    event.key === 'Enter'
      ? new Slice(
          Fragment.from(editor.schema.nodes[event.shiftKey ? 'hardBreak' : 'softBreak'].create()),
          0,
          0,
        )
      : Slice.empty
  const plan = protectedSelectionPlan(editor, slice)
  if (!plan) return false
  event.preventDefault()
  return dispatchImmediateEdit(editor, plan.transaction)
}

export const SelectionEdits = Extension.create({
  name: 'selectionEdits',
  // Native history starts buffering composition at priority 1500, before repairs.
  priority: 1200,
  addOptions: () => ({ singleLine: false }),
  addProseMirrorPlugins() {
    const editor = this.editor,
      singleLine = this.options.singleLine
    let composition:
      | {
          after: PMNode
          inverse: Step[]
          selection: ReturnType<Selection['toJSON']>
        }
      | undefined
    return [
      new Plugin({
        props: {
          handleKeyDown: (_view, event) => selectionKey(editor, event),
          handleTextInput(_view, from, to, text) {
            if (from === to || !text || editor.view.composing || !editor.isEditable) return false
            const plan = protectedSelectionPlan(editor, textSlice(editor, text), {
              from,
              to,
            })
            return plan ? dispatchImmediateEdit(editor, plan.transaction) : false
          },
          handlePaste(_view, _event, slice) {
            if (!editor.isEditable || editor.view.composing) return false
            const cells = cellPastePlan(editor, slice)
            if (cells) return dispatchImmediateEdit(editor, cells.transaction)
            const plan = protectedSelectionPlan(editor, slice)
            if (plan) return dispatchImmediateEdit(editor, plan.transaction)
            if (singleLine) {
              const nodes = clipboardInline(slice, editor, true)
              return dispatchImmediateEdit(
                editor,
                editor.state.tr.replaceSelection(new Slice(Fragment.fromArray(nodes), 0, 0)),
              )
            }
            return false
          },
          handleDrop(view, event, slice, moved) {
            if (!moved || !editor.isEditable || view.composing) return false
            const plan = protectedSelectionPlan(editor)
            if (!plan) return false
            const point = view.posAtCoords({ left: event.clientX, top: event.clientY })
            if (!point) return true
            const { from, to } = editor.state.selection
            if (point.pos >= from && point.pos <= to) return true
            const target = Selection.near(
              plan.transaction.doc.resolve(plan.transaction.mapping.map(point.pos)),
            )
            if (!(target instanceof TextSelection)) return true
            const nodes = clipboardInline(slice, editor)
            plan.transaction.insert(target.from, Fragment.fromArray(nodes))
            plan.transaction.setSelection(
              TextSelection.create(
                plan.transaction.doc,
                target.from + nodes.reduce((size, node) => size + node.nodeSize, 0),
              ),
            )
            return dispatchImmediateEdit(editor, plan.transaction)
          },
          handleDOMEvents: {
            compositionstart() {
              composition = undefined
              if (!editor.isEditable) return false
              const plan = protectedSelectionPlan(editor)
              if (plan) {
                composition = {
                  after: plan.transaction.doc,
                  inverse: plan.transaction.steps
                    .map((step, index) => step.invert(plan.transaction.docs[index]))
                    .reverse(),
                  selection: editor.state.selection.toJSON(),
                }
                dispatchImmediateEdit(editor, plan.transaction)
              }
              return false
            },
            compositionend(_view, event) {
              const pending = composition
              composition = undefined
              // Let the DOM observer flush the native composition before deciding
              // that it was cancelled; restore while the native owner still buffers.
              if (pending && !event.data)
                queueMicrotask(() => {
                  if (editor.isDestroyed || !editor.state.doc.eq(pending.after)) return
                  const transaction = editor.state.tr
                  for (const step of pending.inverse) transaction.step(step)
                  transaction.setSelection(Selection.fromJSON(transaction.doc, pending.selection))
                  dispatchImmediateEdit(editor, transaction)
                })
              return false
            },
            cut(view, event) {
              if (!editor.isEditable || view.composing) return false
              const plan = protectedSelectionPlan(editor)
              if (!plan) return false
              event.preventDefault()
              // No successful copy => no destructive cut.
              if (!event.clipboardData) return true
              const copy = view.serializeForClipboard(editor.state.selection.content())
              event.clipboardData.setData('text/html', copy.dom.innerHTML)
              event.clipboardData.setData('text/plain', copy.text)
              return dispatchImmediateEdit(editor, plan.transaction.setMeta('uiEvent', 'cut'))
            },
            beforeinput(_view, event) {
              if (
                !event.cancelable ||
                event.isComposing ||
                editor.view.composing ||
                !editor.isEditable ||
                editor.state.selection.empty
              )
                return false
              const slice =
                event.inputType === 'insertText' && event.data
                  ? textSlice(editor, event.data)
                  : event.inputType.startsWith('delete')
                    ? Slice.empty
                    : undefined
              if (!slice) return false
              const plan = protectedSelectionPlan(editor, slice)
              if (!plan) return false
              event.preventDefault()
              return dispatchImmediateEdit(editor, plan.transaction)
            },
          },
        },
      }),
    ]
  },
})

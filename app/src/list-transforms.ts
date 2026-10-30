import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { TextSelection } from '@tiptap/pm/state'
import { normalizeWhitespace } from './document-whitespace'
import { dispatchImmediateEdit } from './immediate-edit'
import { inlineTarget, treeTarget } from './object-commands'
import { editorRoot } from './editor-targets'
import { resumeAfterWidget } from './widgets'

export function listItemAt(editor: Editor) {
  const { $from } = editor.state.selection
  for (let depth = $from.depth - 1; depth > 0; depth--) {
    const node = $from.node(depth)
    if (['listItem', 'descriptionItem'].includes(node.type.name))
      return {
        node,
        depth,
        pos: $from.before(depth),
        list: $from.node(depth - 1),
        index: $from.index(depth - 1),
      }
  }
}
export const emptyListItem = (node: PMNode) =>
  node.content.content.every((part) => part.content.size === 0)

export function planListExit(editor: Editor) {
  const item = listItemAt(editor)
  if (!item || !emptyListItem(item.node)) return
  const { $from } = editor.state.selection
  const start = $from.before(item.depth - 1),
    end = $from.after(item.depth - 1)
  const only = item.list.childCount === 1
  const tr = editor.state.tr.delete(
    only ? start : item.pos,
    only ? end : item.pos + item.node.nodeSize,
  )
  resumeAfterWidget(tr, tr.mapping.map(end))
  normalizeWhitespace(tr)
  const commands = editorRoot(editor)
    ? [{ id: 'list.item.exit', params: { target: treeTarget(editor, item.pos) } }]
    : []
  return { transaction: tr, commands }
}
export function planListSplit(editor: Editor) {
  const item = listItemAt(editor)
  const { selection, storedMarks } = editor.state
  const { $from, $to, from, to } = selection
  if (!item || !$from.sameParent($to)) return
  if (emptyListItem(item.node)) return planListExit(editor)
  const description = item.node.type.name === 'descriptionItem'
  if (description && $from.parent.type.name !== 'descriptionDefinition') return
  const tr = editor.state.tr
  if (description) {
    const next = item.node.type.create(null, [
      editor.schema.nodes.descriptionTerm.create(),
      editor.schema.nodes.descriptionDefinition.create(
        null,
        $from.parent.content.cut($to.parentOffset),
      ),
    ])
    tr.delete(from, $from.end())
    const pos = tr.mapping.map(item.pos + item.node.nodeSize)
    tr.insert(pos, next)
    tr.setSelection(TextSelection.create(tr.doc, pos + 2))
  } else {
    tr.delete(from, to).split(from, 2)
    tr.setSelection(TextSelection.create(tr.doc, from + 4))
  }
  tr.setStoredMarks(storedMarks ?? $from.marks())
  const commands = editorRoot(editor)
    ? [{ id: 'list.item.split', params: { target: inlineTarget(editor, from, to) } }]
    : []
  return { transaction: tr, commands }
}
export function planListJoin(editor: Editor, backwards: boolean) {
  const item = listItemAt(editor)
  const { selection } = editor.state
  const { $from } = selection
  if (!item || !selection.empty || item.node.type.name !== 'listItem') return
  const part = $from.index(item.depth)
  if (
    $from.parentOffset !== (backwards ? 0 : $from.parent.content.size) ||
    part !== (backwards ? 0 : item.node.childCount - 1)
  )
    return
  const index = item.index - (backwards ? 1 : 0)
  if (index < 0 || index + 1 >= item.list.childCount) return
  const left = item.list.child(index),
    right = item.list.child(index + 1)
  let pos = $from.before(item.depth - 1) + 1
  for (let n = 0; n < index; n++) pos += item.list.child(n).nodeSize
  const seam = pos + left.nodeSize - 2
  const tr = editor.state.tr
  if (emptyListItem(right)) tr.delete(pos + left.nodeSize, pos + left.nodeSize + right.nodeSize)
  else tr.join(pos + left.nodeSize).join(seam + 1)
  tr.setSelection(TextSelection.create(tr.doc, seam))
  tr.setStoredMarks(editor.state.storedMarks ?? tr.selection.$from.marks())
  const commands = editorRoot(editor)
    ? [
        {
          id: 'list.item.join',
          params: {
            left: treeTarget(editor, pos),
            right: treeTarget(editor, pos + left.nodeSize),
          },
        },
      ]
    : []
  return { transaction: tr, commands }
}

/** Keyboard and contract tests use the same plans; Rust confirms the immediate delta. */
export function listKey(editor: Editor, key: 'Enter' | 'Backspace' | 'Delete') {
  if (editor.view.composing || !editor.isEditable) return false
  const item = listItemAt(editor)
  if (!item) return false
  const { selection } = editor.state
  const { $from } = selection
  if (key === 'Enter') {
    if ($from.parent.type.name === 'descriptionTerm') {
      editor.commands.setTextSelection(item.pos + item.node.child(0).nodeSize + 2)
      return true
    }
    const plan = planListSplit(editor)
    return plan ? dispatchImmediateEdit(editor, plan.transaction) : false
  }
  const plan = planListJoin(editor, key === 'Backspace')
  if (plan) return dispatchImmediateEdit(editor, plan.transaction)
  if (!selection.empty || $from.parentOffset !== 0 || key !== 'Backspace') return false
  if (item.node.type.name === 'descriptionItem' && $from.index(item.depth) === 1) {
    editor.commands.setTextSelection(
      TextSelection.near(editor.state.doc.resolve($from.before()), -1),
    )
    return true
  }
  if ($from.index(item.depth) !== 0) return false
  if (item.index > 0) {
    if (emptyListItem(item.node)) {
      const tr = editor.state.tr.delete(item.pos, item.pos + item.node.nodeSize)
      tr.setSelection(TextSelection.near(tr.doc.resolve(item.pos), -1))
      return dispatchImmediateEdit(editor, tr)
    }
    editor.commands.setTextSelection(item.pos - 2)
  } else {
    const before = $from.before(item.depth - 1)
    if (before > 0)
      editor.commands.setTextSelection(TextSelection.near(editor.state.doc.resolve(before), -1))
  }
  return true
}

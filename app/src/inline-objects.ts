import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { NodeSelection } from '@tiptap/pm/state'

/** Inline atoms are objects, unlike text, marks or nonselectable line breaks. */
export function isInlineObject(node?: PMNode | null): node is PMNode {
  return !!node?.isInline && node.isAtom && NodeSelection.isSelectable(node)
}
export function inlineObjectTarget(editor: Editor, side?: 'before' | 'after') {
  const { selection } = editor.state
  if (selection instanceof NodeSelection)
    return isInlineObject(selection.node)
      ? { pos: selection.from, node: selection.node }
      : undefined
  if (!selection.empty) return
  const { $from } = selection
  if (side !== 'before' && isInlineObject($from.nodeAfter))
    return { pos: $from.pos, node: $from.nodeAfter }
  if (side !== 'after' && isInlineObject($from.nodeBefore))
    return { pos: $from.pos - $from.nodeBefore.nodeSize, node: $from.nodeBefore }
}
export function inlineObjectKey(
  editor: Editor,
  event: KeyboardEvent,
  properties: (pos: number, kind: string, remove?: boolean) => void,
) {
  if (event.isComposing || editor.view.composing) return false
  if (event.key !== 'Backspace' && event.key !== 'Delete') return false
  const target = inlineObjectTarget(editor, event.key === 'Backspace' ? 'before' : 'after')
  if (!target) return false
  event.preventDefault()
  properties(target.pos, target.node.type.name, true)
  return true
}

/** A noneditable, selectable badge with one shared pointer/keyboard contract. */
export function inlineObjectView(options: {
  editor: Editor
  getPos: () => number | undefined
  kind: string
  label: string
  title: string
  className?: string
  open: (pos: number, kind: string) => void
}) {
  const dom = document.createElement('span')
  dom.className = `inline-object ${options.className || ''}`.trim()
  dom.contentEditable = 'false'
  dom.textContent = options.label
  dom.setAttribute('role', 'button')
  dom.tabIndex = 0
  const select = () => {
    const pos = options.getPos()
    if (pos !== undefined) options.editor.commands.setNodeSelection(pos)
    return pos
  }
  const open = () => {
    const pos = select()
    if (pos !== undefined) options.open(pos, options.kind)
  }
  dom.addEventListener('mousedown', (event) => {
    if (event.button === 0) event.preventDefault()
  })
  dom.addEventListener('focus', select)
  dom.addEventListener('click', open)
  dom.addEventListener('keydown', (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      event.stopPropagation()
      open()
    }
  })
  const describe = (text: string, number?: number) => {
    const suffix = number === undefined ? '' : ` ${number}`
    const label = `${options.label}${suffix}`
    if (dom.textContent !== label) dom.textContent = label
    const title = `${options.title}${suffix}`
    dom.title = text ? `${title} : ${text}` : title
    dom.setAttribute('aria-label', dom.title)
  }
  return { dom, describe }
}

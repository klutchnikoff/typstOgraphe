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
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    if (event.shiftKey || event.altKey || event.ctrlKey || event.metaKey) return false
    const backwards = event.key === 'ArrowLeft'
    const target = inlineObjectTarget(editor, backwards ? 'before' : 'after')
    if (!target) return false
    event.preventDefault()
    editor.commands.setTextSelection(target.pos + (backwards ? 0 : target.node.nodeSize))
    editor.view.focus()
    return true
  }
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
  // Keep the browser's insertion boundary outside the painted badge.
  const label = document.createElement('span')
  label.className = 'inline-object-label'
  label.textContent = options.label
  label.contentEditable = 'false'
  label.setAttribute('role', 'button')
  label.tabIndex = 0
  dom.append(label)
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
  label.addEventListener('focus', select)
  dom.addEventListener('click', (event) => {
    // The leading gap is a text insertion target, not part of the badge button.
    const rect = label.getBoundingClientRect()
    const inGap =
      getComputedStyle(dom).direction === 'rtl'
        ? event.clientX > rect.right
        : event.clientX < rect.left
    if (inGap && event.detail > 0) {
      const pos = options.getPos()
      if (pos !== undefined) {
        options.editor.commands.setTextSelection(pos)
        options.editor.view.focus()
      }
    } else open()
  })
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
    const textLabel = `${options.label}${suffix}`
    if (label.textContent !== textLabel) label.textContent = textLabel
    const title = `${options.title}${suffix}`
    label.title = text ? `${title} : ${text}` : title
    label.setAttribute('aria-label', label.title)
  }
  return { dom, describe }
}

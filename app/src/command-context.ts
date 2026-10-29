import type { Editor } from '@tiptap/core'
import type { Capability } from './commands'

const owners = new WeakMap<Editor, Editor>()
const current = new WeakMap<Editor, Editor>()
const lost = new WeakSet<Editor>()
const links = new WeakMap<Editor, () => void>()
const catalogues = new WeakMap<
  Editor,
  { entries: Capability[]; available: () => string[] | undefined }
>()
export function bindCommandCapabilities(
  editor: Editor,
  entries: Capability[],
  available: () => string[] | undefined,
) {
  catalogues.set(editor, { entries, available })
}
export function commandCapabilities(editor: Editor) {
  return catalogues.get(owners.get(editor) || editor)
}

/** Keep the last editorial zone while a menu/palette owns DOM focus. Detached
 * modal drafts belong to themselves and can never retarget the live document.
 */
export function bindCommandContext(editor: Editor, owner = editor, openLink?: () => void) {
  owners.set(editor, owner)
  if (openLink) links.set(editor, openLink)
  const focus = () => {
    lost.delete(owner)
    current.set(owner, editor)
    document.dispatchEvent(new Event('commands:refresh'))
  }
  editor.on('focus', focus)
  // Badges and widget headers can open a band before the text view receives
  // focus. Their explicit pointer/keyboard activation still selects this zone.
  editor.view.dom.addEventListener('pointerdown', focus, true)
  editor.view.dom.addEventListener('click', focus, true)
  editor.on('transaction', () => {
    if (current.get(owner) === editor) document.dispatchEvent(new Event('commands:refresh'))
  })
  editor.on('destroy', () => {
    editor.view.dom.removeEventListener('pointerdown', focus, true)
    editor.view.dom.removeEventListener('click', focus, true)
    if (current.get(owner) === editor) {
      current.delete(owner)
      if (owner !== editor) lost.add(owner)
      document.dispatchEvent(new Event('commands:refresh'))
    }
  })
}
export function hasCommandTarget(editor: Editor) {
  return !lost.has(owners.get(editor) || editor)
}
export function commandEditor(editor: Editor): Editor {
  const owner = owners.get(editor) || editor
  const target = current.get(owner)
  return target && !target.isDestroyed ? target : owner
}
export function openCommandLink(editor: Editor) {
  links.get(editor)?.()
}

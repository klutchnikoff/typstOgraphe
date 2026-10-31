import type { Editor } from '@tiptap/core'
import { documentContent } from './shared-inline-marks'
import { privateDrafts } from './private-drafts'

const owners = new WeakMap<Editor, object>()
export const formOwner = (editor: Editor) => owners.get(editor)
export const bindFormOwner = (editor: Editor, owner: object) => owners.set(editor, owner)

/** One collector for contextual bands and detached forms. No validation or submission. */
export function trackForm(
  owner: object,
  element: HTMLElement,
  context: unknown,
  data: () => unknown = () => null,
) {
  const read = () => ({
    fields: [
      ...element.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
        'input, textarea, select',
      ),
    ].map((input, index) => {
      if (input instanceof HTMLInputElement && input.type === 'file' && input.files?.length)
        throw new Error('Recovery file fields require an explicit resource payload.')
      return {
        id: input.id || input.name || `field-${index}`,
        type: input.type,
        value: input.value,
        ...(input instanceof HTMLInputElement ? { checked: input.checked } : {}),
        ...(input instanceof HTMLSelectElement
          ? { selected: [...input.selectedOptions].map((o) => o.value) }
          : {}),
      }
    }),
    choices: [...element.querySelectorAll<HTMLElement>('[aria-checked]')].map((choice, index) => ({
      id: choice.id || `choice-${index}`,
      value: choice.getAttribute('aria-checked'),
    })),
    rich: [...element.querySelectorAll<HTMLElement>('[contenteditable=true]')].flatMap(
      (node, index) => {
        const editor = (node as HTMLElement & { editor?: Editor }).editor
        return editor
          ? [{ id: node.id || `rich-${index}`, document: documentContent(editor.getJSON()) }]
          : []
      },
    ),
    data: data(),
  })
  const handle = privateDrafts(owner).track(element.id, context, read, () => element.isConnected)
  const changed = () => handle.changed()
  const begin = () => handle.beginComposition()
  const end = () => queueMicrotask(() => handle.endComposition())
  for (const name of ['input', 'change', 'click', 'private-draft:input'])
    element.addEventListener(name, changed)
  element.addEventListener('compositionstart', begin, true)
  element.addEventListener('compositionend', end)
  const editors = [...element.querySelectorAll<HTMLElement>('[contenteditable=true]')].flatMap(
    (node) => {
      const editor = (node as HTMLElement & { editor?: Editor }).editor
      return editor ? [editor] : []
    },
  )
  for (const editor of editors) editor.on('update', changed)
  const dispose = (discard: boolean) => {
    if (discard) handle.discard()
    else handle.detach()
    for (const name of ['input', 'change', 'click', 'private-draft:input'])
      element.removeEventListener(name, changed)
    element.removeEventListener('compositionstart', begin, true)
    element.removeEventListener('compositionend', end)
    for (const editor of editors) editor.off('update', changed)
  }
  return { changed, discard: () => dispose(true), detach: () => dispose(false) }
}

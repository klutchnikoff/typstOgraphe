import { tr } from './i18n'
import type { Editor } from '@tiptap/core'
import type { ContextPanel } from './context-panel'
import type { FormDraft } from './private-drafts'
import type { Provider } from './provider'
import { consumeRecovered, type RecoveredItem } from './recovered-work'
import { editorRoot, positionAtPath } from './editor-targets'
import { documentContent } from './shared-inline-marks'
import { stableJSON } from './provider'
const panels = new WeakMap<HTMLElement, ContextPanel>()
export const bindRecoveryPanel = (panel: ContextPanel) => panels.set(panel.element, panel)
/** Replay only into an unpublished UI. Application still uses its ordinary command path. */
export function hydrateForm(provider: Provider, element: HTMLElement, item: RecoveredItem) {
  const panel = panels.get(element)
  if (!panel) return false
  const snapshot = item.payload as FormDraft,
    data = snapshot.data as any
  if (!data || !Array.isArray(data.fields)) return false
  if (!panel.restoreRecoveryData(data.data)) return false
  const controls = [
    ...element.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
      'input, textarea, select',
    ),
  ]
  for (const [index, value] of data.fields.entries()) {
    const input = controls.find((c) => c.id && c.id === value.id) || controls[index]
    if (!input || input.type !== value.type || input.type === 'file') continue
    input.value = String(value.value ?? '')
    if (input instanceof HTMLInputElement && input.type === 'checkbox')
      input.checked = !!value.checked
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.dispatchEvent(new Event('change', { bubbles: true }))
  }
  for (const [index, value] of (data.rich || []).entries()) {
    const nodes = [...element.querySelectorAll<HTMLElement>('[contenteditable=true]')]
    const node = nodes.find((n) => n.id === value.id) || nodes[index]
    const editor = (node as HTMLElement & { editor?: Editor })?.editor
    if (editor) editor.commands.setContent(value.document)
  }
  if (!panel.element.querySelector('.recovery-form-notice')) {
    const notice = document.createElement('p')
    notice.className = 'recovery-form-notice'
    notice.textContent = tr('Saisie récupérée · à valider')
    panel.body.prepend(notice)
  }
  panel.bindRecoverySource(item.id)
  panel.recoveryApplied = () => consumeRecovered(provider, item.id)
  panel.recoveryFailed = (error) => provider.onError(error)
  panel.focus()
  return true
}
/** Mounted zones register their existing opener; no duplicate editor or widget renderer. */
type Opener = (position: number, draft: FormDraft) => boolean
const openers = new WeakMap<object, Map<Editor, Opener>>()
export function registerRecoveryOpener(provider: object, editor: Editor, open: Opener) {
  let map = openers.get(provider)
  if (!map) {
    map = new Map()
    openers.set(provider, map)
  }
  map.set(editor, open)
  editor.on('destroy', () => map!.delete(editor))
}
export function reopenBand(provider: Provider, item: RecoveredItem) {
  const draft = item.payload as FormDraft,
    context = draft.context as any
  for (const [editor, open] of openers.get(provider) || []) {
    if (stableJSON(editorRoot(editor) || { kind: 'body' }) !== stableJSON(context.root)) continue
    try {
      let pos = context.range.from
      if (typeof context.offset === 'number') {
        try {
          pos = positionAtPath(editor, context.path) + context.offset
        } catch {
          if (context.range.from !== context.range.to) continue
        }
      }
      const size = context.range.to - context.range.from
      if (pos + size > editor.state.doc.content.size) continue
      if (
        stableJSON(documentContent(editor.state.doc.slice(pos, pos + size).toJSON())) !==
        stableJSON(documentContent(context.original))
      )
        continue
      if (!open(pos, draft)) continue
      const panel = [...document.querySelectorAll<HTMLElement>('form.context-band')].find(
        (el) => !el.hidden && panels.has(el),
      )
      if (panel) return hydrateForm(provider, panel, item)
    } catch {
      /* Missing or changed targets stay available as isolated drafts. */
    }
  }
  return false
}

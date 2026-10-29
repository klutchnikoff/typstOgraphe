import type { Editor } from '@tiptap/core'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import type { Provider } from './provider'
import { bindEditingContext } from './editing-context'

const zones = new WeakMap<Provider, Map<Editor, { editable: boolean }>>()
const owners = new WeakMap<Editor, Provider>()
export const editingProvider = (editor: Editor) => owners.get(editor)
/** A native-bound zone has NO Yjs UndoManager; all its entry points use Rust. */
export function bindEditingHistory(editor: Editor, provider: Provider) {
  owners.set(editor, provider)
  bindEditingContext(editor, provider)
  let entries = zones.get(provider)
  if (!entries) {
    entries = new Map()
    zones.set(provider, entries)
    let focused: Editor | undefined
    provider.onHistoryLock = (locked) => {
      if (locked) focused = [...entries!.keys()].find((zone) => !zone.isDestroyed && zone.isFocused)
      for (const [zone, state] of entries!) {
        if (zone.isDestroyed) continue
        if (locked) state.editable = zone.isEditable
        zone.setEditable(locked ? false : state.editable, false)
      }
      if (
        !locked &&
        focused &&
        !focused.isDestroyed &&
        (document.activeElement === document.body || document.activeElement === focused.view.dom)
      )
        focused.view.focus()
    }
  }
  entries.set(editor, { editable: editor.isEditable })
  editor.on('destroy', () => entries!.delete(editor))
}
export function historyFor(editor: Editor) {
  const provider = owners.get(editor)
  if (!provider) return yUndoPluginKey.getState(editor.state)?.undoManager
  return {
    stopCapturing: () => provider.stopCapturing(),
    clear: () => {
      void provider.clearHistory().catch(provider.onError)
    },
    undo: () => {
      void provider.history('undo').catch(provider.onError)
    },
    redo: () => {
      void provider.history('redo').catch(provider.onError)
    },
    get undoStack() {
      return { length: Number(provider.canHistory()) }
    },
    get redoStack() {
      return { length: Number(provider.canHistory(true)) }
    },
  }
}

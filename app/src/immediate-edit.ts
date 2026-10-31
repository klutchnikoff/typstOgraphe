import type { Editor } from '@tiptap/core'
import type { Transaction } from '@tiptap/pm/state'
import { editingProvider } from './editing-history'
import { editingView } from './editing-context'

/** Publish immediate input with its pre-edit context in the same queued delta.
 * No native round trip before painting, and no second JavaScript undo stack. */
export function dispatchImmediateEdit(editor: Editor, transaction: Transaction) {
  if (editor.isDestroyed || !editor.isEditable) return false
  const provider = editingProvider(editor)
  const before = editingView(editor)
  const dispatch = () => editor.view.dispatch(transaction.scrollIntoView())
  if (provider && before) provider.withHistoryView(before, dispatch)
  else dispatch()
  return true
}

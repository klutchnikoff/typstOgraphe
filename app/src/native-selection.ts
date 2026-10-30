import type { Editor } from '@tiptap/core'
import { TextSelection } from '@tiptap/pm/state'

/** Native navigation may precede its queued selectionchange notification.
 * Read that caret before a keyboard command, rather than applying a style at
 * the previous position and losing it when the notification finally arrives.
 */
export function syncNativeTextSelection(editor: Editor) {
  if (
    editor.isDestroyed ||
    editor.view.composing ||
    !editor.view.hasFocus() ||
    !(editor.state.selection instanceof TextSelection)
  )
    return
  const view = editor.view,
    dom = view.dom.ownerDocument.getSelection()
  if (
    !dom?.anchorNode ||
    !dom.focusNode ||
    !view.dom.contains(dom.anchorNode) ||
    !view.dom.contains(dom.focusNode)
  )
    return
  const selection = TextSelection.between(
    view.state.doc.resolve(view.posAtDOM(dom.anchorNode, dom.anchorOffset)),
    view.state.doc.resolve(view.posAtDOM(dom.focusNode, dom.focusOffset)),
  )
  // Reapplying an unchanged selection would itself clear the typing styles.
  if (!selection.eq(view.state.selection)) view.dispatch(view.state.tr.setSelection(selection))
}

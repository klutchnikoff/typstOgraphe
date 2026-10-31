import type { Editor } from '@tiptap/core'
import { yCursorPlugin, yCursorPluginKey } from '@tiptap/y-tiptap'
import type { LocalPresence } from './local-presence'

/** Mount the optional cursor plugin; reconfigure the existing view, never its history. */
export function bindPresenceCaret(editor: Editor, presence: LocalPresence) {
  let installed: typeof presence.awareness
  const release = presence.observe(() => {
    if (editor.isDestroyed || installed === presence.awareness) return
    if (installed) editor.unregisterPlugin(yCursorPluginKey)
    installed = presence.awareness
    if (installed)
      editor.registerPlugin(
        yCursorPlugin(installed, {
          cursorBuilder: (user) => {
            // LocalPresence supplies the two palette colors. Never inject remote CSS.
            const color = /^#[0-9a-fA-F]{6}$/.test(user.color) ? user.color : '#286b91'
            const caret = document.createElement('span')
            caret.className = 'collaboration-carets__caret'
            caret.style.borderColor = color
            const label = document.createElement('span')
            label.className = 'collaboration-carets__label'
            label.style.backgroundColor = color
            label.textContent = String(user.name || '')
            caret.append(label)
            return caret
          },
        }),
      )
  })
  editor.on('destroy', release)
}

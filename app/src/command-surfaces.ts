import type { Editor } from '@tiptap/core'
import type { Capability, Registry } from './commands'
import { refreshCommands } from './commands'
import { commandEditor } from './command-context'
import { editingAvailability, allowed, refused } from './command-availability'
import { commandAvailability, setCommandPolicy } from './public-commands'
import type { Transport } from './provider'

/** One policy for every public trigger, including native menus. Native text
 * inputs retain their own undo/selection; palettes retain their originating zone.
 */
export function mountCommandSurfaces(options: {
  editor: Editor
  commands: Registry
  capabilities: Capability[]
  requiresDocument: Set<string>
  active: () => boolean
  native?: Transport
  onError: (error: unknown) => void
}) {
  const { editor, commands, native } = options
  const editorial = new Set([
    ...options.capabilities.map((c) => c.id),
    ...options.capabilities.flatMap((c) => c.editor?.actions?.map((a) => a.id) || []),
    'history.undo',
    'history.redo',
    'text.clear',
    'text.unlink',
    'context.open',
    'block.remove',
  ])
  let nativeTextField = false,
    destroyed = false
  setCommandPolicy(commands, (command) => {
    if (document.querySelector('dialog[open]') || document.getElementById('app')?.inert)
      return refused('modal')
    if (
      (options.requiresDocument.has(command.id) || editorial.has(command.id)) &&
      !options.active()
    )
      return refused('no_document')
    if (!editorial.has(command.id)) return allowed
    // Pointer/native-menu undo belongs to the document. Keyboard undo in a
    // native input is separately left to that input by the shell dispatcher.
    if (nativeTextField && !command.id.startsWith('history.')) return refused('text_required')
    const target = commandEditor(editor)
    if (
      target !== editor &&
      (command.id.startsWith('table.') || ['context.open', 'block.remove'].includes(command.id))
    )
      return refused('body_required')
    return editingAvailability(target, !command.id.startsWith('history.'))
  })
  const refresh = (event?: Event) => {
    if (event instanceof CustomEvent && event.detail === commandEditor(editor))
      nativeTextField = false
    refreshCommands(commands)
  }
  const focused = (event: FocusEvent) => {
    const target = event.target
    if (target instanceof Element && target.closest('[data-rich-zone-profile]'))
      nativeTextField = false
    else if (
      (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) &&
      !target.matches('.command-palette-input')
    )
      nativeTextField = true
    refresh()
  }
  let desired = '',
    published = '',
    publishing = false,
    focusEpoch = 0
  const publish = async () => {
    if (!native || publishing || destroyed) return
    publishing = true
    try {
      while (!destroyed && desired !== published) {
        const value = desired
        const epoch = focusEpoch
        await native.invoke('set_menu_availability', { states: JSON.parse(value) })
        // A background window may have been ignored natively. A foreground
        // event during this request must still publish its current menu state.
        published = epoch === focusEpoch ? value : ''
      }
    } catch (error) {
      if (!destroyed) options.onError(error)
    } finally {
      publishing = false
    }
  }
  const syncNative = () => {
    if (!native) return
    desired = JSON.stringify(
      Object.fromEntries(
        [...commands].map(([id, command]) => [id, commandAvailability(command).enabled]),
      ),
    )
    void publish()
  }
  const foreground = () => {
    focusEpoch++
    published = ''
    syncNative()
  }
  document.addEventListener('commands:refresh', refresh)
  document.addEventListener('focusin', focused)
  document.addEventListener('commands:availability', syncNative)
  window.addEventListener('focus', foreground)
  const observer = new MutationObserver(() => refresh())
  observer.observe(document.body, {
    subtree: true,
    attributes: true,
    attributeFilter: ['open', 'hidden', 'inert'],
  })
  return {
    refresh,
    destroy() {
      destroyed = true
      observer.disconnect()
      document.removeEventListener('commands:refresh', refresh)
      document.removeEventListener('focusin', focused)
      document.removeEventListener('commands:availability', syncNative)
      window.removeEventListener('focus', foreground)
    },
  }
}

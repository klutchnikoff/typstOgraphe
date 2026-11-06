import { commandPalette } from './command-palette'
import { keyboardScope } from './keyboard-navigation'
import { tr } from './i18n'
import { searchState } from './proofing'
import { runCommand } from './public-commands'
import { searchPanel } from './search-panel'
import type { ShellContext } from './shell-context'
import { byId, type ShellLayout } from './shell-layout'
import { sideRailMode } from './side-rail'
import type { versionsPanel } from './versions-panel'

export type SidePanels = ReturnType<typeof sidePanels>

/**
 * Owns the side panel: only one view is shown at a time, and the views that hold
 * subscriptions (versions, recovered inputs) are released whenever it changes.
 */
export function sidePanels(ctx: ShellContext, layout: Pick<ShellLayout, 'side' | 'panel'>) {
  const { editor, commands } = ctx
  let palette: ReturnType<typeof commandPalette> | undefined
  let versions: ReturnType<typeof versionsPanel> | undefined
  let releaseRecovered: (() => void) | undefined
  const release = () => {
    palette?.destroy()
    palette = undefined
    versions?.destroy()
    versions = undefined
    releaseRecovered?.()
    releaseRecovered = undefined
    searchState(editor, '')
  }
  const close = () => {
    release()
    railMode.close()
    editor.commands.focus()
  }
  const railMode = sideRailMode(layout.side, layout.panel, {
    toggle: () => runCommand(commands.get('ui.sidebar_labels')!),
    close,
    resized: (width) => {
      ctx.state.preferences.sidebar_width = width
      ctx.guarded(() => ctx.persistPreferences())
    },
  })
  byId('close-side').onclick = close
  keyboardScope(layout.panel, { close, priority: 10, tab: true })
  /** Opens the panel on `section` and returns its emptied content element. */
  function show(title: string, section?: string) {
    release()
    railMode.show(section)
    byId('side-title').textContent = title
    const content = byId('side-content')
    content.replaceChildren()
    return content
  }
  return {
    railMode,
    close,
    show,
    adoptVersions(panel: ReturnType<typeof versionsPanel>) {
      versions = panel
    },
    refreshVersions() {
      void versions?.refresh()
    },
    /** Closes the versions view, which belongs to the document being left. */
    dismissVersions() {
      if (!versions) return
      versions.destroy()
      versions = undefined
      railMode.close()
    },
    adoptRecovered(next: () => void) {
      releaseRecovered = next
    },
    releaseRecovered() {
      releaseRecovered?.()
    },
    palette() {
      palette = commandPalette(show(tr('Commandes'), 'ui.palette'), commands, (command) => {
        release()
        railMode.close()
        runCommand(command)
      })
    },
    search() {
      searchPanel(show(tr('Rechercher et remplacer'), 'ui.find'), editor)
    },
  }
}

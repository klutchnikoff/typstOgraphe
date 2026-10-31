import { tr } from './i18n'
import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'

/** Native recipe controls follow the same enabled/acknowledged states as a person.
 * A CRDT event can expose new text before the command's IPC reply unlocks the UI.
 */
export async function until(check: () => boolean | Promise<boolean>, label: string) {
  const deadline = Date.now() + 30000
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(`Closure timeout: ${label}`)
    await new Promise((resolve) => setTimeout(resolve, 50))
  }
}

export async function clickControl(root: Element, selector: string, label: string) {
  let button: HTMLButtonElement | null = null
  await until(() => {
    button = root.querySelector<HTMLButtonElement>(selector)
    return !!button?.isConnected && !button.disabled && !button.closest('[inert]')
  }, label)
  button!.click()
}

export function comparisonDialog() {
  const dialogs = document.querySelectorAll<HTMLDialogElement>('dialog.version-dialog[open]')
  if (dialogs.length !== 1)
    throw new Error(`Expected one comparison dialog, found ${dialogs.length}`)
  return dialogs[0]
}

export async function finishComparison(dialog: HTMLDialogElement) {
  await until(
    () =>
      dialog.querySelector('[data-comparison-summary]')?.textContent ===
      tr('Choix appliqués. Vous pouvez enregistrer le document.'),
    'comparison application acknowledgement',
  )
  await clickControl(dialog, '.version-footer .primary', 'comparison close enabled')
  await until(() => !dialog.isConnected, 'comparison modal close')
}

export async function historyStep(
  editor: Editor,
  provider: Provider,
  direction: 'undo' | 'redo',
  matches: () => boolean,
  label: string,
) {
  await until(
    () => editor.isEditable && provider.canHistory(direction === 'redo'),
    `${label} available`,
  )
  editor.view.focus()
  if (!editor.commands[direction]()) throw new Error(`${label} unavailable`)
  await until(matches, label)
  await provider.flush(false)
  await until(() => !provider.historyBusy && editor.isEditable, `${label} acknowledged`)
}

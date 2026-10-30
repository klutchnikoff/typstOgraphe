import { afterEach, expect, it, vi } from 'vitest'
import { createRichZone } from '../src/rich-zone'
import { createRegistry } from '../src/commands'
import { mountCommandSurfaces } from '../src/command-surfaces'
import { commandAvailability } from '../src/public-commands'
import type { Editor } from '@tiptap/core'
import type { Transport } from '../src/provider'
import { bindCommandContext } from '../src/command-context'

let editor: Editor, surfaces: ReturnType<typeof mountCommandSurfaces>
afterEach(() => {
  surfaces?.destroy()
  editor?.destroy()
  document.body.replaceChildren()
})
async function mount(native?: Transport) {
  const element = document.createElement('div')
  document.body.append(element)
  editor = createRichZone({
    profile: 'document',
    element,
    label: 'Document',
    content: '<p>Texte</p>',
  })
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const commands = createRegistry(editor, [], vi.fn(), vi.fn())
  const run = vi.fn()
  commands.set('text.clear', { id: 'text.clear', label: 'Effacer', run })
  let active = true
  surfaces = mountCommandSurfaces({
    editor,
    commands,
    capabilities: [],
    requiresDocument: new Set(),
    active: () => active,
    native,
    onError: (error) => {
      throw error
    },
  })
  return {
    commands,
    run,
    inactive: () => {
      active = false
    },
  }
}
it('native input undo is left alone, and the palette cannot retarget its text action to the document', async () => {
  const { commands, run, inactive } = await mount()
  const input = document.createElement('input'),
    palette = document.createElement('input')
  palette.className = 'command-palette-input'
  document.body.append(input, palette)
  input.focus()
  palette.focus()
  expect(commandAvailability(commands.get('text.clear')!)).toMatchObject({
    reason: 'text_required',
  })
  commands.get('text.clear')!.run()
  expect(run).not.toHaveBeenCalled()
  // An explicit editorial selection also chooses the body without stealing DOM focus.
  editor.commands.setTextSelection(2)
  expect(document.activeElement).toBe(palette)
  commands.get('text.clear')!.run()
  expect(run).toHaveBeenCalledOnce()
  inactive()
  expect(commandAvailability(commands.get('text.clear')!)).toMatchObject({ reason: 'no_document' })
})
it('modal state blocks direct public actions and is recomputed after cancellation', async () => {
  const { commands, run } = await mount()
  const dialog = document.createElement('dialog')
  dialog.setAttribute('open', '')
  document.body.append(dialog)
  expect(commandAvailability(commands.get('text.clear')!)).toMatchObject({ reason: 'modal' })
  commands.get('text.clear')!.run()
  expect(run).not.toHaveBeenCalled()
  dialog.remove()
  commands.get('text.clear')!.run()
  expect(run).toHaveBeenCalledOnce()
})
it('document undo stays available after a search field takes focus, while native input keystrokes keep their own route', async () => {
  const { commands } = await mount()
  const undo = vi.fn()
  commands.set('history.undo', {
    id: 'history.undo',
    label: 'Annuler',
    run: undo,
    enabled: () => true,
  })
  const search = document.createElement('input')
  document.body.append(search)
  search.focus()
  expect(commandAvailability(commands.get('history.undo')!).enabled).toBe(true)
  commands.get('history.undo')!.run()
  expect(undo).toHaveBeenCalledOnce()
  expect(commandAvailability(commands.get('text.clear')!).enabled).toBe(false)
})
it('removing the last rich field invalidates text targets but never the document history', async () => {
  const { commands } = await mount()
  const undo = vi.fn()
  commands.set('history.undo', {
    id: 'history.undo',
    label: 'Annuler',
    run: undo,
    enabled: () => true,
  })
  const element = document.createElement('div')
  document.body.append(element)
  const field = createRichZone({
    profile: 'rich-text',
    element,
    label: 'Résumé',
    content: '<p>Résumé</p>',
  })
  if (!field.isInitialized)
    await new Promise<void>((resolve) => field.once('create', () => resolve()))
  bindCommandContext(field, editor)
  field.view.focus()
  field.destroy()
  expect(commandAvailability(commands.get('text.clear')!).reason).toBe('no_selection')
  commands.get('history.undo')!.run()
  expect(undo).toHaveBeenCalledOnce()
})
it('native menu publication is ordered, coalesces changes and sends nothing when availability is unchanged', async () => {
  let release!: () => void
  const calls: any[] = []
  const invoke: Transport['invoke'] = async (command, args) => {
    calls.push({ command, args })
    if (calls.length === 1)
      await new Promise<void>((resolve) => {
        release = resolve
      })
    return undefined as any
  }
  const { inactive } = await mount({ invoke } as Transport)
  surfaces.refresh()
  surfaces.refresh()
  expect(calls).toHaveLength(1)
  expect(calls[0].command).toBe('set_menu_availability')
  expect(calls[0].args.states['text.clear']).toBe(true)
  inactive()
  surfaces.refresh()
  surfaces.refresh()
  expect(calls).toHaveLength(1)
  release()
  await vi.waitFor(() => expect(calls).toHaveLength(2))
  expect(calls[1].args.states['text.clear']).toBe(false)
  surfaces.refresh()
  await Promise.resolve()
  expect(calls).toHaveLength(2)
  window.dispatchEvent(new Event('focus'))
  await vi.waitFor(() => expect(calls).toHaveLength(3))
})
it('regaining foreground during an outstanding native reply still refreshes the menu', async () => {
  let release!: () => void
  const invoke = vi.fn(async () => {
    if (invoke.mock.calls.length === 1)
      await new Promise<void>((resolve) => {
        release = resolve
      })
  })
  await mount({ invoke } as unknown as Transport)
  surfaces.refresh()
  window.dispatchEvent(new Event('focus'))
  release()
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2))
})

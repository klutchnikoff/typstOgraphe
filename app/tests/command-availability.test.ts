import { afterEach, expect, it, vi } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone, type RichZoneProfile } from '../src/rich-zone'
import { bindCommandContext, commandEditor } from '../src/command-context'
import { createRegistry, toolbar, refreshCommands, type Capability } from '../src/commands'
import { commandPalette } from '../src/command-palette'
import { commandAvailability, runCommand, setCommandPolicy } from '../src/public-commands'
import { interactionAvailability, allowed, refused } from '../src/command-availability'
import { applyFormatting } from '../src/formatting'

const editors: Editor[] = []
afterEach(() => {
  editors
    .splice(0)
    .reverse()
    .forEach((e) => e.destroy())
  document.body.replaceChildren()
})
async function zone(profile: RichZoneProfile = 'document', content = '<p>Texte</p>') {
  const element = document.createElement('div')
  document.body.append(element)
  const editor = createRichZone({ profile, element, label: 'Zone', content })
  editors.push(editor)
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  return editor
}
function capabilities(): Capability[] {
  return [
    ['text.bold', 'Gras', 'inline', 'style'],
    ['text.link', 'Lien', 'inline', 'link'],
    ['block.heading_1', 'Titre 1', 'headings', 'heading'],
    ['block.quote', 'Citation', 'insert', 'block'],
    ['block.bullet', 'Liste', 'lists', 'list'],
    ['insert.footnote', 'Note', 'insert', 'inline'],
  ].map(([id, label, group, availability]) => ({
    id,
    label,
    group,
    label_key: id,
    icon: 'pilcrow',
    node: null,
    mark: id === 'text.bold' ? 'bold' : null,
    letter_only: false,
    parameters: {},
    editor: { availability: availability as 'style' },
  }))
}
it('toolbar, palette and direct invocation all refuse an invalid title without editing a widget', async () => {
  const editor = await zone('document', '<blockquote><p>Texte</p></blockquote>')
  editor.commands.setTextSelection(2)
  const registry = createRegistry(editor, capabilities(), vi.fn(), vi.fn())
  const bar = document.createElement('div'),
    panel = document.createElement('div')
  document.body.append(bar, panel)
  toolbar(bar, registry, capabilities())
  refreshCommands(registry)
  expect(commandAvailability(registry.get('block.heading_1')!)).toEqual(
    refused('top_level_required'),
  )
  expect(bar.querySelector<HTMLButtonElement>('[data-command="block.heading_1"]')!.disabled).toBe(
    true,
  )
  commandPalette(panel, registry, (c) => runCommand(c))
  const option = [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Titre 1')!
  expect(option.disabled).toBe(true)
  expect(option.title).toContain('widget')
  const before = editor.getJSON()
  registry.get('block.heading_1')!.run()
  expect(await applyFormatting(editor, 'block.heading_1')).toBe(false)
  editor.commands.keyboardShortcut('Mod-Alt-1')
  expect(editor.getJSON()).toEqual(before)
  expect(registry.get('text.bold')!.enabled!()).toBe(true)
})
it('the last rich field remains the target after palette focus; body blocks are unavailable there', async () => {
  const body = await zone(),
    field = await zone('rich-text', '<p>Résumé</p>')
  bindCommandContext(field, body)
  field.commands.setTextSelection({ from: 1, to: 7 })
  field.view.focus()
  expect(commandEditor(body)).toBe(field)
  const registry = createRegistry(body, capabilities(), vi.fn(), vi.fn())
  const panel = document.createElement('div')
  document.body.append(panel)
  commandPalette(panel, registry, (c) => runCommand(c))
  expect(document.activeElement).toBe(panel.querySelector('input'))
  expect(registry.get('block.quote')!.availability!()).toEqual(refused('body_required'))
  const before = body.getJSON()
  expect(runCommand(registry.get('text.bold')!)).toBe(true)
  expect(field.view.dom.querySelector('strong')?.textContent).toBe('Résumé')
  expect(body.getJSON()).toEqual(before)
  field.destroy()
  expect(commandAvailability(registry.get('text.bold')!)).toEqual(refused('no_selection'))
  body.view.dom.dispatchEvent(new MouseEvent('click', { bubbles: true }))
  expect(commandAvailability(registry.get('text.bold')!).enabled).toBe(true)
  body.view.focus()
  expect(commandEditor(body)).toBe(body)
  expect(commandAvailability(registry.get('text.bold')!).enabled).toBe(true)
})
it('link opening, caret styles, read-only zones and IME use the same interaction guard', async () => {
  const editor = await zone()
  expect(interactionAvailability(editor, 'style')).toEqual(allowed)
  expect(interactionAvailability(editor, 'link')).toEqual(refused('selection_required'))
  editor.commands.setTextSelection({ from: 1, to: 3 })
  expect(interactionAvailability(editor, 'link')).toEqual(allowed)
  editor.commands.setLink({ href: 'https://example.org' })
  editor.commands.setTextSelection(2)
  expect(interactionAvailability(editor, 'link')).toEqual(allowed)
  const composing = vi.spyOn(editor.view, 'composing', 'get').mockReturnValue(true)
  expect(await applyFormatting(editor, 'text.bold')).toBe(false)
  composing.mockRestore()
  editor.setEditable(false)
  expect(interactionAvailability(editor, 'style')).toEqual(refused('read_only'))
})
it('global policy also guards direct runs and updates an already-open palette', async () => {
  const editor = await zone(),
    registry = createRegistry(editor, capabilities(), vi.fn(), vi.fn())
  let active = true
  setCommandPolicy(registry, () => (active ? allowed : refused('no_document')))
  const panel = document.createElement('div')
  document.body.append(panel)
  commandPalette(panel, registry, (c) => runCommand(c))
  active = false
  refreshCommands(registry)
  expect([...panel.querySelectorAll('button')].every((b) => b.disabled)).toBe(true)
  const before = editor.getJSON()
  registry.get('block.quote')!.run()
  expect(editor.getJSON()).toEqual(before)
  active = true
  editor.commands.setTextSelection({ from: 1, to: 3 })
  refreshCommands(registry)
  expect(
    [...panel.querySelectorAll('button')].find((b) => b.textContent === 'Gras')!.disabled,
  ).toBe(false)
})

import { afterEach, expect, it, vi } from 'vitest'
import type { Editor } from '@tiptap/core'
import { createRichZone } from '../src/rich-zone'
import { createRegistry, toolbar, refreshCommands, type Capability } from '../src/commands'
import { commandPalette } from '../src/command-palette'
import { commandAvailability, runCommand } from '../src/public-commands'

let editor: Editor | undefined
let palette: ReturnType<typeof commandPalette> | undefined
afterEach(() => {
  palette?.destroy()
  editor?.destroy()
  document.body.replaceChildren()
})

it('derives all surfaces and child actions from capabilities, across arbitrary type switches', async () => {
  const element = document.createElement('div')
  document.body.append(element)
  editor = createRichZone({ profile: 'document', element, label: 'Body', content: '<p>Text</p>' })
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor!.once('create', () => resolve()))
  const capabilities: Capability[] = [
    {
      id: 'text.bold',
      group: 'inline',
      label: 'Bold',
      mark: 'bold',
      editor: { availability: 'style' },
    },
    { id: 'letter.insert_postscript', group: 'letter', label: 'Postscript' },
    {
      id: 'insert.table',
      group: 'insert',
      label: 'Table',
      editor: {
        actions: [
          {
            id: 'table.add_row',
            label: 'Row',
            command: 'add',
            axis: 'row',
            offset: 1,
            remove: false,
          },
        ],
      },
    },
  ].map((item) => ({
    label_key: item.id,
    icon: 'pilcrow',
    node: null,
    mark: null,
    parameters: {},
    ...item,
  })) as Capability[]
  // This fixture is neither Article nor Courrier: a future type assembled from
  // existing capabilities must need no changes in a toolbar/palette adapter.
  let supported = ['text.bold']
  const registry = createRegistry(editor, capabilities, vi.fn(), vi.fn(), () => supported)
  const row = vi.fn(),
    global = vi.fn()
  registry.set('table.add_row', { id: 'table.add_row', label: 'Row', run: row })
  registry.set('ui.preferences', { id: 'ui.preferences', label: 'Preferences', run: global })
  const bar = document.createElement('div'),
    panel = document.createElement('div')
  document.body.append(bar, panel)
  toolbar(bar, registry, capabilities)
  palette = commandPalette(panel, registry, runCommand)
  const labels = () => [...panel.querySelectorAll('button')].map((item) => item.textContent)
  expect(labels()).toContain('Bold')
  expect(labels()).toContain('Preferences')
  expect(labels()).not.toContain('Postscript')
  expect(labels()).not.toContain('Row')
  expect(bar.querySelector('[data-command-group="letter"]')).toHaveProperty('hidden', true)
  expect(bar.querySelector('[data-command-group="headings"]')).toHaveProperty('hidden', true)
  registry.get('table.add_row')!.run()
  expect(row).not.toHaveBeenCalled()
  // A supported capability remains shown when only its editing context is invalid.
  editor.setEditable(false)
  refreshCommands(registry)
  expect(labels()).toContain('Bold')
  expect(commandAvailability(registry.get('text.bold')!).enabled).toBe(false)
  expect(panel.querySelector('button')).toHaveProperty('disabled', true)
  expect(runCommand(registry.get('ui.preferences')!)).toBe(true)
  expect(global).toHaveBeenCalledOnce()
  const input = panel.querySelector('input')!
  input.value = 'Post'
  input.dispatchEvent(new Event('input'))
  supported = ['letter.insert_postscript', 'insert.table']
  refreshCommands(registry)
  expect(input.value).toBe('Post')
  expect(document.activeElement).toBe(input)
  expect(labels()).toEqual(['Postscript'])
  expect(bar.querySelector('[data-command-group="letter"]')).toHaveProperty('hidden', false)
  input.value = ''
  input.dispatchEvent(new Event('input'))
  expect(labels()).toContain('Row')
  expect(labels()).not.toContain('Bold')
  expect(runCommand(registry.get('table.add_row')!)).toBe(true)
  expect(row).toHaveBeenCalledOnce()
  supported = []
  refreshCommands(registry)
  expect(labels()).toContain('Preferences')
  expect(labels()).not.toContain('Row')
  // The shell observes these attributes to schedule interface refreshes. A
  // stable refresh must not feed its own observer and keep requesting frames.
  const observer = new MutationObserver(() => {})
  observer.observe(bar, { subtree: true, attributes: true, attributeFilter: ['hidden', 'open'] })
  refreshCommands(registry)
  expect(observer.takeRecords()).toHaveLength(0)
  observer.disconnect()
})

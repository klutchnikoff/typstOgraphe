import { navigationGroup } from './keyboard-navigation'
import { tr } from './i18n'
import { runFormatting, type Formatting } from './formatting'
import { insertInlineObject } from './object-commands'
import { editingProvider } from './editing-history'
import { CommandRegistry, refreshCommandButton, runCommand } from './public-commands'
import {
  capabilityAvailability,
  refused,
  type Availability,
  type InteractionPolicy,
} from './command-availability'
import { bindCommandCapabilities, commandEditor, openCommandLink } from './command-context'
import type { Editor, JSONContent } from '@tiptap/core'
import type { Parameters } from './capability-settings'
import { insertList, insertQuote, insertBlock } from './editing'
export interface TableAction {
  id: string
  label: string
  command: string
  axis: 'row' | 'column'
  offset: number
  remove: boolean
}
export interface Capability {
  id: string
  label: string
  label_key: string
  icon: string
  group: string
  node: string | null
  mark: string | null
  letter_only: boolean
  parameters: Parameters
  editor?: { conversion_group?: string; actions?: TableAction[]; availability?: InteractionPolicy }
}
export interface PublicCommand {
  id: string
  label: string
  icon?: string
  shortcut?: string
  availability?: () => Availability
  enabled?: () => boolean
  active?: () => boolean
  run: () => void
}
export type Registry = Map<string, PublicCommand>
export { icon } from './icons'
import { icon } from './icons'
export function createRegistry(
  editor: Editor,
  capabilities: Capability[],
  properties: (pos: number, kind: string) => void,
  image: () => void,
  available: () => string[] | undefined = () => undefined,
  citation: () => void = () => {},
  table: () => void = () => {},
): Registry {
  const registry: Registry = new CommandRegistry()
  bindCommandCapabilities(editor, capabilities, available)
  const target = () => commandEditor(editor)
  const actions: Record<string, () => void> = {
    'block.paragraph': () => {
      runFormatting(target(), 'block.paragraph')
    },
    'text.bold': () => {
      runFormatting(target(), 'text.bold')
    },
    'text.italic': () => {
      runFormatting(target(), 'text.italic')
    },
    'text.link': () => openCommandLink(target()),
    'block.bullet': () => {
      insertList(target(), 'bulletList')
    },
    'block.numbered': () => {
      insertList(target(), 'orderedList')
    },
    'block.description': () => {
      insertList(target(), 'descriptionList')
    },
    'block.quote': () => {
      insertQuote(target())
    },
    'insert.image': image,
    'insert.table': table,
    'insert.citation': citation,
    'insert.footnote': () =>
      insertInlineWithSettings(target(), { type: 'footnote', attrs: { text: '' } }, properties),
    'letter.insert_postscript': () => {
      insertBlock(target(), { type: 'postscript' })
    },
    'letter.insert_attachment': () => {
      insertBlock(target(), { type: 'attachment' })
    },
  }
  for (const capability of capabilities) {
    const heading = /block.heading_(\d)/.exec(capability.id)
    const run = heading
      ? () => {
          runFormatting(target(), capability.id as Formatting)
        }
      : actions[capability.id]
    if (!run) throw new Error(`Capacité sans adaptateur web : ${capability.id}`)
    registry.set(capability.id, {
      ...capability,
      run,
      availability: () => {
        if (available()?.includes(capability.id) === false)
          return refused('unsupported_document_type')
        return capabilityAvailability(
          target(),
          capability.id,
          capability.editor?.availability || 'block',
        )
      },
      active: () =>
        heading
          ? target().isActive('heading', { level: Number(heading[1]) })
          : capability.mark
            ? target().isActive(capability.mark)
            : !!capability.node && target().isActive(capability.node),
    })
  }
  for (const [id, label, shortcut, action] of [
    ['history.undo', tr('Undo'), 'Mod+Z', 'undo'],
    ['history.redo', tr('Rétablir'), 'Mod+Shift+Z', 'redo'],
  ] as const)
    registry.set(id, {
      id,
      label,
      shortcut,
      icon: action === 'undo' ? 'undo-2' : 'redo-2',
      run: () => {
        target().chain().focus()[action]().run()
      },
      enabled: () =>
        !target().isDestroyed &&
        target().isEditable &&
        !target().view.composing &&
        target().can()[action](),
    })
  return registry
}
export function toolbar(element: HTMLElement, registry: Registry, capabilities: Capability[]) {
  element.replaceChildren()
  navigationGroup(element, { axis: 'horizontal' })
  const button = (command: PublicCommand) => {
    const el = document.createElement('button')
    el.type = 'button'
    el.setAttribute('aria-label', command.label)
    el.title = command.label
    el.dataset.command = command.id
    el.innerHTML = icon(command.icon || 'pilcrow')
    // Le clic d'une commande de mise en forme conserve la sélection du texte.
    el.onmousedown = (event) => {
      if (event.button === 0) event.preventDefault()
    }
    el.onclick = () => {
      if (!runCommand(command)) return
      el.closest('details')?.removeAttribute('open')
    }
    return el
  }
  const group = (id: string, label: string, iconName: string) => {
    const menu = document.createElement('details')
    menu.className = 'tool-menu'
    const summary = document.createElement('summary')
    summary.setAttribute('role', 'button')
    summary.setAttribute('aria-label', label)
    summary.title = label
    summary.innerHTML = icon(iconName)
    const panel = document.createElement('div')
    panel.className = 'tool-menu-content'
    for (const capability of capabilities.filter((c) => c.group === id)) {
      const command = registry.get(capability.id)!
      const entry = button(command)
      const text = document.createElement('span')
      text.textContent = command.label
      entry.append(text)
      panel.append(entry)
    }
    menu.append(summary, panel)
    element.append(menu)
  }
  group('headings', tr('Titres'), 'heading')
  for (const capability of capabilities.filter((c) => c.group === 'inline'))
    element.append(button(registry.get(capability.id)!))
  group('lists', tr('Listes'), 'list')
  for (const capability of capabilities.filter((c) => c.group === 'insert'))
    element.append(button(registry.get(capability.id)!))
  group('letter', tr('Fin de lettre'), 'paperclip')
  const spacer = document.createElement('span')
  spacer.className = 'toolbar-spacer'
  element.append(spacer)
  for (const id of ['history.undo', 'history.redo']) element.append(button(registry.get(id)!))
}
export function refreshCommands(registry: Registry) {
  for (const el of document.querySelectorAll<HTMLButtonElement>('button[data-command]')) {
    const command = registry.get(el.dataset.command!)
    if (command) refreshCommandButton(el, command)
    if (command?.active) el.setAttribute('aria-pressed', String(command.active()))
  }
  document.dispatchEvent(new Event('commands:availability'))
}
export function positionAtPath(editor: Editor, path: number[]): number {
  let node = editor.state.doc,
    pos = 0
  for (const [depth, index] of path.entries()) {
    if (!Number.isInteger(index) || index < 0 || index >= node.childCount)
      throw new Error('Cible structurée invalide.')
    if (depth) pos++
    for (let i = 0; i < index; i++) pos += node.child(i).nodeSize
    node = node.child(index)
  }
  return pos
}
export function insertNode(editor: Editor, node: JSONContent) {
  if (node.type && editor.schema.nodes[node.type]?.isBlock) return insertBlock(editor, node)
  void insertInlineObject(editor, node).catch((error: unknown) =>
    editingProvider(editor)?.onError(error),
  )
  return true
}

/** Inline tools transfer focus to their band, never schedule editor.focus() first. */
export function insertInlineWithSettings(
  editor: Editor,
  content: JSONContent,
  open: (pos: number, kind: string) => void,
) {
  const node = editor.schema.nodeFromJSON(content)
  if (!node.isInline) return false
  void insertInlineObject(editor, content, open).catch((error: unknown) =>
    editingProvider(editor)?.onError(error),
  )
  return true
}

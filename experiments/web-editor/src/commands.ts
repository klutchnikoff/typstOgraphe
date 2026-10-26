import { tr } from './i18n'
import type { Editor, JSONContent } from '@tiptap/core'
import { insertList, insertQuote } from './editing'
export interface Capability {
  id: string
  label: string
  label_key: string
  icon: string
  group: string
  node: string | null
  mark: string | null
  letter_only: boolean
  parameters: unknown
}
export interface PublicCommand {
  id: string
  label: string
  icon?: string
  shortcut?: string
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
): Registry {
  const registry: Registry = new Map()
  const actions: Record<string, () => void> = {
    'block.paragraph': () => {
      editor.chain().focus().setParagraph().run()
    },
    'text.bold': () => {
      editor.chain().focus().toggleBold().run()
    },
    'text.italic': () => {
      editor.chain().focus().toggleItalic().run()
    },
    'text.link': () => properties(editor.state.selection.from, 'link'),
    'block.bullet': () => {
      insertList(editor, 'bulletList')
    },
    'block.numbered': () => {
      insertList(editor, 'orderedList')
    },
    'block.description': () => {
      insertList(editor, 'descriptionList')
    },
    'block.quote': () => {
      insertQuote(editor)
    },
    'insert.image': image,
    'insert.table': () => {
      editor.chain().focus().insertTable({ rows: 3, cols: 2, withHeaderRow: true }).run()
    },
    'insert.footnote': () => {
      editor
        .chain()
        .focus()
        .insertContent({ type: 'footnote', attrs: { text: '' } })
        .run()
      properties(editor.state.selection.from - 1, 'footnote')
    },
    'letter.insert_postscript': () => {
      editor.chain().focus().insertContent({ type: 'postscript' }).run()
    },
    'letter.insert_attachment': () => {
      editor.chain().focus().insertContent({ type: 'attachment' }).run()
    },
  }
  for (const capability of capabilities) {
    const heading = /block.heading_(\d)/.exec(capability.id)
    const run = heading
      ? () => {
          editor
            .chain()
            .focus()
            .toggleHeading({ level: Number(heading[1]) as 1 | 2 | 3 })
            .run()
        }
      : actions[capability.id]
    if (!run) throw new Error(`Capacité sans adaptateur web : ${capability.id}`)
    registry.set(capability.id, {
      ...capability,
      run,
      enabled: () => available()?.includes(capability.id) ?? true,
      active: () =>
        heading
          ? editor.isActive('heading', { level: Number(heading[1]) })
          : capability.mark
            ? editor.isActive(capability.mark)
            : !!capability.node && editor.isActive(capability.node),
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
        editor.chain().focus()[action]().run()
      },
      enabled: () => editor.can()[action](),
    })
  return registry
}
export function toolbar(element: HTMLElement, registry: Registry, capabilities: Capability[]) {
  element.replaceChildren()
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
      if (command.enabled?.() === false) return
      command.run()
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
    el.disabled = command?.enabled?.() === false
    if (command?.active) el.setAttribute('aria-pressed', String(command.active()))
  }
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
  editor.chain().focus().insertContent(node).run()
}

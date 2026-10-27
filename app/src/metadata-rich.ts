import { Editor, Extension } from '@tiptap/core'
import { ySyncPlugin, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { editorialExtensions } from './editorial-schema'
import { BandHost } from './band-layout'
import { propertyPanel } from './properties'
import { icon } from './icons'
import { navigationGroup } from './keyboard-navigation'
import { tr } from './i18n'
import type { Catalogue } from './shell'

/** Named rich zone: Yjs binding, shared undo owner, and the existing link band. */
export function metadataRich(
  parent: HTMLElement,
  label: string,
  fragment: Y.XmlFragment,
  inline: boolean,
  main: Editor,
  catalogue: Catalogue,
  language: string,
) {
  const root = document.createElement('div')
  root.className = `metadata-rich ${inline ? 'metadata-rich-inline' : ''}`
  const heading = document.createElement('div')
  heading.className = 'metadata-rich-heading'
  const title = document.createElement('span')
  title.textContent = label
  const tools = document.createElement('div')
  tools.className = 'metadata-rich-tools'
  navigationGroup(tools, { axis: 'horizontal' })
  const input = document.createElement('div')
  heading.append(title, tools)
  root.append(heading, input)
  parent.append(root)
  const history = yUndoPluginKey.getState(main.state)?.undoManager
  let properties: ReturnType<typeof propertyPanel>
  const buttons = new Map<string, HTMLButtonElement>()
  let editor!: Editor
  for (const [mark, text, glyph] of [
    ['bold', 'Gras', 'bold'],
    ['italic', 'Italique', 'italic'],
    ['link', 'Lien', 'link'],
  ]) {
    const button = document.createElement('button')
    button.type = 'button'
    button.title = `${tr(text)} · ${label}`
    button.setAttribute('aria-label', button.title)
    button.innerHTML = icon(glyph)
    button.onmousedown = (event) => event.preventDefault()
    button.onclick = () => {
      history?.stopCapturing()
      if (mark === 'link') properties.open(editor.state.selection.from, 'link')
      else if (mark === 'bold') editor.chain().focus().toggleBold().run()
      else editor.chain().focus().toggleItalic().run()
    }
    tools.append(button)
    buttons.set(mark, button)
  }
  const refresh = (current: Editor) => {
    for (const [mark, button] of buttons) {
      button.setAttribute('aria-pressed', String(current.isActive(mark)))
      if (mark === 'link')
        button.disabled = current.state.selection.empty && !current.isActive('link')
    }
  }
  const sharedHistory = (redo = false) => {
    history?.stopCapturing()
    if (redo) history?.redo()
    else history?.undo()
    return true
  }
  editor = new Editor({
    element: input,
    extensions: [
      ...editorialExtensions(inline),
      Extension.create({
        name: 'editorialSharedDocument',
        addProseMirrorPlugins: () => [ySyncPlugin(fragment)],
        addKeyboardShortcuts: () => ({
          'Mod-k': () => {
            if (editor.state.selection.empty && !editor.isActive('link')) return true
            properties.open(editor.state.selection.from, 'link')
            return true
          },
          'Mod-z': () => sharedHistory(),
          'Mod-Shift-z': () => sharedHistory(true),
          'Mod-y': () => sharedHistory(true),
        }),
      }),
    ],
    editorProps: {
      attributes: {
        role: 'textbox',
        'aria-label': label,
        'aria-multiline': String(!inline),
        lang: language,
        spellcheck: 'false',
        autocorrect: 'off',
      },
      handleKeyDown: (_view, event) => {
        if (
          !event.isComposing &&
          (event.ctrlKey || event.metaKey) &&
          event.key === 'Enter' &&
          editor.isActive('link')
        ) {
          event.preventDefault()
          properties.open(editor.state.selection.from, 'link')
          return true
        }
        return false
      },
    },
    onTransaction: ({ editor: current }) => refresh(current),
    onFocus: () => history?.stopCapturing(),
    onBlur: () => history?.stopCapturing(),
  })
  const bands = new BandHost(editor, parent.closest<HTMLElement>('#metadata-content')!, input, main)
  properties = propertyPanel(editor, bands, catalogue, () => {}, `metadata-${crypto.randomUUID()}-`)
  refresh(editor)
  return {
    updateLanguage(value: string) {
      editor.view.dom.lang = value
    },
    focus: () => editor.commands.focus(),
    destroy: () => editor.destroy(),
  }
}

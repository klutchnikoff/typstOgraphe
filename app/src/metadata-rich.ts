import { editingProvider } from './editing-history'
import { bindFormOwner, formOwner } from './form-drafts'
import { historyFor } from './editing-history'
import { capabilityAvailability } from './command-availability'
import { runFormatting } from './formatting'
import type { RecordRoot } from './editor-targets'
import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import { createRichZone, richZoneProfiles, type EditorialProfile } from './rich-zone'
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
  profile: EditorialProfile,
  main: Editor,
  catalogue: Catalogue,
  language: string,
  target: RecordRoot,
) {
  const inline = !richZoneProfiles[profile].multiline
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
  const history = historyFor(main)
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
      if (
        !capabilityAvailability(editor, `text.${mark}`, mark === 'link' ? 'link' : 'style').enabled
      )
        return
      if (mark === 'link') properties.open(editor.state.selection.from, 'link')
      else runFormatting(editor, mark === 'bold' ? 'text.bold' : 'text.italic')
    }
    tools.append(button)
    buttons.set(mark, button)
  }
  const refresh = (current: Editor) => {
    for (const [mark, button] of buttons) {
      button.setAttribute('aria-pressed', String(current.isActive(mark)))
      button.disabled = !capabilityAvailability(
        current,
        `text.${mark}`,
        mark === 'link' ? 'link' : 'style',
      ).enabled
    }
  }
  editor = createRichZone({
    profile,
    element: input,
    label,
    language,
    binding: { kind: 'fragment', fragment, historyOwner: main, target },
    openLink: (zone) => properties.open(zone.state.selection.from, 'link'),
    onTransaction: ({ editor: current }) => refresh(current),
  })
  const recoveryOwner = editingProvider(main) || formOwner(main)
  if (recoveryOwner) bindFormOwner(editor, recoveryOwner)
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

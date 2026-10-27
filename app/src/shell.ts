import { commandPalette } from './command-palette'
import { searchPanel, navigateSearch } from './search-panel'
import { preferencesPanel, type Preferences } from './preferences-panel'
import { tr } from './i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import * as Y from 'yjs'
import type { Editor } from '@tiptap/core'
import { yUndoPluginKey } from '@tiptap/y-tiptap'
import type { Provider, Transport, Resource } from './provider'
import type { Capability, Registry } from './commands'
import { icon } from './commands'
import { mountAssistance } from './assistance'
import { mountProofing, searchState, navigateSpelling } from './proofing'
import type { BandHost } from './band-layout'
import { documentType, settingsFor, setSetting, setDocument, type Settings } from './state'
import { welcomeBrand } from './welcome-brand'
import { recoveryPanel } from './recovery-panel'
export interface Model {
  id: string
  label: string
  description: string
  source: string
  fixedPage: { paper: string; landscape: boolean } | null
}
export interface Field {
  field: string
  parameter: string
  label: string
  value: string
  group: string
  multiline: boolean
  kind: string
  required_for_export: boolean
}
export interface DocumentType {
  capabilities: string[]
  id: string
  label: string
  canCreate: boolean
  settingsHeading: string
  metadata: Settings['metadata']
  fields: Field[]
  groups: { id: string; label: string }[]
  models: Model[]
}
export interface Catalogue {
  messages: Record<string, string>
  locale: string
  types: DocumentType[]
  entries: Capability[]
  papers: { value: string; label: string }[]
  languages: { value: string; label: string }[]
}

interface Shell {
  bands: BandHost
  editor: Editor
  provider: Provider
  transport: Transport
  catalogue: Catalogue
  commands: Registry
  save: () => Promise<void>
  schedule: () => void
  message: (s: string, error?: boolean) => void
  isTauri: boolean
  hasRecovery?: boolean
  opened?: boolean
  filePath?: string | null
  initialPath?: string | null
  diagnostic: boolean
}
export async function mountShell(s: Shell) {
  const { editor, provider, transport, catalogue, commands } = s
  const settings = provider.doc.getMap('settings')
  let active = !!(s.opened || s.diagnostic),
    dirty = false,
    path: string | null = s.filePath || null,
    savedRevision = provider.revision
  let preferences: Preferences = (await transport.invoke('preferences_load').catch(() => ({
    appearance: 'system',
    font: 'serif',
    size: 18,
    profile: {},
    signature: null,
    recent: [],
    personal: {},
    editor_fraction: 0.5,
    spelling_enabled: true,
  }))) as Preferences
  const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T
  document.querySelector('.badge')?.remove()
  $('rust-experiment').hidden = !s.diagnostic
  $('presence').hidden = !s.diagnostic
  const header = document.querySelector('header')!
  const oldTools = header.querySelector<HTMLElement>('.session-tools')
  if (oldTools) oldTools.hidden = true
  const menu = document.createElement('nav')
  menu.className = 'app-menu'
  header.append(menu)
  if (s.isTauri && navigator.platform.startsWith('Mac')) {
    menu.hidden = true
    const brand = header.querySelector('.identity')
    if (brand) document.querySelector('#toolbar')!.prepend(brand)
    header.hidden = true
  }
  const workspace = document.querySelector('main')!
  const welcome = document.createElement('section')
  welcome.className = 'welcome'
  welcome.innerHTML =
    '<div class="welcome-card"><div class="welcome-actions"><button id="create-document" class="primary">Nouveau document</button><button id="open-document">Ouvrir…</button><button id="resume-document">Reprendre la session</button></div></div>'
  const brand = welcomeBrand()
  welcome.querySelector('.welcome-card')!.prepend(brand.element)
  workspace.before(welcome)
  $<HTMLButtonElement>('resume-document').disabled = !s.hasRecovery
  for (const [id, label] of [
    ['create-document', 'Nouveau document'],
    ['open-document', 'Ouvrir…'],
    ['resume-document', 'Reprendre la session'],
  ])
    $(id).textContent = tr(label)

  const side = document.createElement('aside')
  side.className = 'side-rail'
  side.setAttribute('aria-label', 'Panneaux de l’application')
  workspace.append(side)
  const panel = document.createElement('aside')
  panel.className = 'side-panel'
  panel.hidden = true
  panel.innerHTML =
    '<div class="panel-title"><h2 id="side-title"></h2><button id="close-side" aria-label="Fermer le panneau">×</button></div><div id="side-content"></div>'
  workspace.append(panel)
  $('close-side').onclick = () => {
    searchState(editor, '')
    panel.hidden = true
    editor.commands.focus()
  }
  const overlay = document.createElement('dialog')
  overlay.className = 'app-dialog'
  document.body.append(overlay)
  const context = document.createElement('section')
  context.className = 'document-settings'
  context.innerHTML =
    '<div class="metadata-tabs"><strong id="document-type-label"></strong><div id="metadata-tabs"></div></div><div id="metadata-content" hidden></div>'
  document.querySelector('.document-info')!.replaceWith(context)
  let tab: string | null = null
  const guarded = (task: () => Promise<unknown> | void) => {
    try {
      const result = task()
      if (result instanceof Promise) void result.catch((e) => s.message(String(e), true))
    } catch (e) {
      s.message(String(e), true)
    }
  }
  const requiresDocument = new Set([
    'document.save',
    'document.save_as',
    'document.export',
    'document.close',
    'ui.find',
    'find.next',
    'find.previous',
    'text.clear',
    'text.unlink',
    'spelling.next',
    'spelling.previous',
    'spelling.reset',
    'ui.information',
  ])
  const add = (id: string, label: string, run: () => void, shortcut?: string) =>
    commands.set(id, {
      id,
      label,
      run,
      shortcut,
      enabled: () => !requiresDocument.has(id) || active,
    })
  const setActive = (value: boolean) => {
    active = value
    welcome.hidden = value
    workspace.hidden = !value
    if (value) brand.finish()
    else brand.play()
    if (value) {
      editor.commands.focus()
      s.schedule()
    }
    updateTitle()
  }
  function updateTitle() {
    const title =
      path
        ?.split(/[\\/]/)
        .at(-1)
        ?.replace(/\.typsto$/, '') || tr('Sans titre')
    if (s.isTauri)
      void transport.invoke('set_window_title', {
        title: active ? `${title}${dirty ? ' *' : ''}` : 'typstOgraphe',
      })
    header.dataset.document = title
  }
  function confirmDiscard(next: () => void) {
    if (!active || !dirty) {
      next()
      return
    }
    overlay.replaceChildren()
    const heading = document.createElement('h2')
    heading.textContent = tr('Conserver les modifications ?')
    const text = document.createElement('p')
    text.textContent = tr(
      'Enregistrez le document avant de le remplacer, ou abandonnez les modifications de cette session.',
    )
    const buttons = document.createElement('div')
    buttons.className = 'panel-actions'
    for (const [label, style, action] of [
      [
        tr('Abandonner'),
        'danger',
        () => {
          overlay.close()
          next()
        },
      ],
      [tr('Annuler'), '', () => overlay.close()],
      [
        tr('Enregistrer'),
        'primary',
        () =>
          guarded(async () => {
            if (!(await saveFile())) return
            overlay.close()
            next()
          }),
      ],
    ] as const) {
      const b = document.createElement('button')
      b.textContent = label
      b.className = style
      b.onclick = action
      buttons.append(b)
    }
    overlay.append(heading, text, buttons)
    overlay.showModal()
  }
  async function saveFile(as = false) {
    await provider.flush()
    if (!s.isTauri) {
      await s.save()
      dirty = false
      return true
    }
    const destination =
      as || !path ? await transport.invoke<string | null>('choose_file', { save: true }) : path
    if (!destination) return false
    await transport.invoke('save_file', {
      path: destination,
      snapshot: {
        revision: provider.revision,
        document: editor.getJSON(),
        settings: settingsFor(provider),
      },
      update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
    })
    path = destination
    dirty = false
    savedRevision = provider.revision
    await remember(destination)
    updateTitle()
    s.message(tr('Document enregistré.'))
    return true
  }
  async function remember(file: string) {
    preferences.recent = [file, ...preferences.recent.filter((p) => p !== file)].slice(0, 12)
    await transport.invoke('preferences_save', { preferences })
  }
  async function open(file?: string) {
    const selected = file || (await transport.invoke<string | null>('choose_file', { save: false }))
    if (!selected) return
    await provider.flush()
    const opened = await transport.invoke<{
      document: unknown
      settings: Settings
      resources: Resource[]
    }>('open_file', { path: selected, revision: provider.revision })
    if (s.isTauri) {
      await remember(selected)
      return
    }
    for (const resource of opened.resources) provider.addResource(resource)
    provider.doc.transact(() => {
      setDocument(provider, opened.settings)
      editor.commands.setContent(opened.document as Parameters<Editor['commands']['setContent']>[0])
    })
    path = selected
    dirty = false
    setActive(true)
    await remember(selected)
    yUndoPluginKey.getState(editor.state)?.undoManager.clear()
    renderMetadata()
  }
  function create() {
    overlay.replaceChildren()
    const heading = document.createElement('h2')
    heading.textContent = tr('Nouveau document')
    const types = catalogue.types.filter((t) => t.canCreate)
    const type = document.createElement('select')
    type.setAttribute('aria-label', tr('Type de document'))
    for (const t of types) type.add(new Option(t.label, t.id))
    const model = document.createElement('select')
    model.setAttribute('aria-label', tr('Modèle'))
    const fill = () => {
      model.replaceChildren()
      for (const m of types.find((t) => t.id === type.value)!.models)
        model.add(new Option(m.label, m.id))
    }
    type.onchange = fill
    fill()
    const controls = document.createElement('div')
    controls.className = 'panel-actions'
    const cancel = document.createElement('button')
    cancel.textContent = tr('Annuler')
    cancel.onclick = () => overlay.close()
    const submit = document.createElement('button')
    submit.className = 'primary'
    submit.textContent = tr('Créer')
    submit.onclick = () => {
      const t = types.find((t) => t.id === type.value)!,
        m = t.models.find((m) => m.id === model.value)!
      const metadata = structuredClone(t.metadata)
      for (const f of t.fields)
        if (f.field in preferences.profile)
          metadata.fields[f.parameter] = preferences.profile[f.field]
      const date = t.fields.find((f) => f.kind === 'date')
      if (date) {
        const now = new Date()
        metadata.fields[date.parameter] =
          `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
      }
      if (s.isTauri) {
        if (preferences.signature) metadata.fields.signature = preferences.signature.path
        guarded(async () => {
          await provider.flush()
          await transport.invoke('new_document', {
            settings: { metadata, template: m.source, modelId: m.id },
            revision: provider.revision,
          })
        })
        return
      }
      provider.doc.transact(() => {
        setDocument(provider, { metadata, template: m.source, modelId: m.id })
        editor.commands.clearContent()
      })
      if (preferences.signature) {
        provider.addResource(preferences.signature)
        setSetting(provider, 'signature', preferences.signature.path)
      }
      path = null
      dirty = false
      yUndoPluginKey.getState(editor.state)?.undoManager.clear()
      overlay.close()
      setActive(true)
      renderMetadata()
    }
    controls.append(cancel, submit)
    overlay.append(heading, type, model, controls)
    overlay.showModal()
  }
  function renderMetadata() {
    if (!settings.has('document')) return
    const kind = documentType(provider),
      current = settingsFor(provider)
    // Native text services and accessibility follow the document language;
    // menus and other interface controls keep the Fluent language on <html>.
    editor.view.dom.lang = String(current.metadata.fields.options.language)
    $('document-type-label').textContent = kind.settingsHeading
    const tabs = $('metadata-tabs')
    tabs.replaceChildren()
    for (const group of [{ id: 'general', label: tr('Général') }, ...kind.groups]) {
      const button = document.createElement('button')
      button.textContent = group.label
      button.setAttribute('aria-pressed', String(tab === group.id))
      button.onclick = () => {
        tab = tab === group.id ? null : group.id
        renderMetadata()
      }
      tabs.append(button)
    }
    const content = $('metadata-content')
    content.hidden = tab === null
    if (content.contains(document.activeElement)) return
    content.replaceChildren()
    if (!tab) return
    const field = (label: string, control: HTMLElement) => {
      const row = document.createElement('label')
      row.className = 'field'
      const name = document.createElement('span')
      name.textContent = label
      row.append(name, control)
      content.append(row)
    }
    if (tab === 'general') {
      const model = document.createElement('select')
      model.setAttribute('aria-label', tr('Modèle'))
      for (const m of kind.models) model.add(new Option(m.label, m.id))
      if (current.modelId === 'embedded') model.add(new Option(tr('Modèle embarqué'), 'embedded'))
      model.value = current.modelId
      model.onchange = () => {
        const selected = kind.models.find((m) => m.id === model.value)!
        const next = settingsFor(provider)
        next.modelId = selected.id
        next.template = selected.source
        setDocument(provider, next)
        renderMetadata()
      }
      field(tr('Modèle'), model)
      const importModel = document.createElement('input')
      importModel.type = 'file'
      importModel.accept = '.typ'
      importModel.setAttribute('aria-label', tr('Importer un modèle Typst'))
      importModel.onchange = () =>
        guarded(async () => {
          const file = importModel.files?.[0]
          if (!file) return
          if (file.size > 2 * 1024 * 1024) throw new Error(tr('Modèle trop volumineux.'))
          await provider.flush()
          const revision = provider.revision
          const next = settingsFor(provider)
          next.modelId = 'embedded'
          next.template = await file.text()
          await transport.invoke('validate_model', {
            snapshot: { revision, document: editor.getJSON(), settings: next },
          })
          if (provider.revision !== revision)
            throw new Error(tr('Le document a changé ; recommencez l’import.'))
          setDocument(provider, next)
          renderMetadata()
        })
      field(tr('Importer un modèle Typst'), importModel)
      for (const [key, label, items] of [
        ['paper', tr('Format'), catalogue.papers],
        ['language', tr('Langue du document'), catalogue.languages],
      ] as const) {
        const select = document.createElement('select')
        select.setAttribute('aria-label', label)
        for (const item of items) select.add(new Option(item.label, item.value))
        select.value = String(current.metadata.fields.options[key])
        select.disabled =
          key === 'paper' && !!kind.models.find((m) => m.id === current.modelId)?.fixedPage
        select.onchange = () => setSetting(provider, `options.${key}`, select.value)
        field(label, select)
      }
      const orientation = document.createElement('select')
      orientation.add(new Option(tr('Portrait'), 'false'))
      orientation.add(new Option(tr('Paysage'), 'true'))
      orientation.value = String(current.metadata.fields.options.landscape)
      orientation.disabled = !!kind.models.find((m) => m.id === current.modelId)?.fixedPage
      orientation.onchange = () =>
        setSetting(provider, 'options.landscape', orientation.value === 'true')
      field(tr('Orientation'), orientation)
      const bullets = document.createElement('p')
      bullets.textContent = tr('Automatique (modèle)')
      field(tr('Puces des listes'), bullets)
    } else
      for (const definition of kind.fields.filter((f) => f.group === tab)) {
        const input = definition.multiline
          ? document.createElement('textarea')
          : document.createElement('input')
        input.setAttribute('aria-label', definition.label)
        if (input instanceof HTMLInputElement)
          input.type = definition.kind === 'date' ? 'date' : 'text'
        input.value = String(current.metadata.fields[definition.parameter] || '')
        input.oninput = () => setSetting(provider, definition.field, input.value)
        input.onblur = () => renderMetadata()
        field(definition.label, input)
        if (definition.kind === 'date') {
          const today = document.createElement('button')
          today.textContent = tr('Aujourd’hui')
          today.onclick = () => {
            const now = new Date()
            setSetting(
              provider,
              definition.field,
              `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`,
            )
            renderMetadata()
          }
          content.append(today)
        }
      }
    if (tab === 'sender') {
      const actions = document.createElement('div')
      actions.className = 'panel-actions'
      for (const [label, value] of [
        [tr('Sans signature'), null],
        [tr('Signature des préférences'), preferences.signature?.path || null],
      ] as const) {
        const button = document.createElement('button')
        button.textContent = label
        button.onclick = () => {
          if (value && preferences.signature) provider.addResource(preferences.signature)
          setSetting(provider, 'signature', value)
        }
        actions.append(button)
      }
      const image = document.createElement('input')
      image.type = 'file'
      image.accept = 'image/png,image/jpeg,image/svg+xml'
      image.setAttribute('aria-label', tr('Signature du document'))
      image.onchange = () =>
        guarded(async () => {
          const file = image.files?.[0]
          if (!file) return
          const resource = await transport.invoke<Resource>('import_image', {
            extension: file.name.split('.').pop(),
            data: Array.from(new Uint8Array(await file.arrayBuffer())),
          })
          provider.addResource(resource)
          setSetting(provider, 'signature', resource.path)
        })
      content.append(actions, image)
    }
  }
  function showPanel(title: string) {
    searchState(editor, '')
    panel.hidden = false
    $('side-title').textContent = title
    const content = $('side-content')
    content.replaceChildren()
    return content
  }
  function palette() {
    commandPalette(showPanel(tr('Commandes')), commands, (command) => {
      panel.hidden = true
      command.run()
    })
  }

  function search() {
    searchPanel(showPanel(tr('Rechercher et remplacer')), editor)
  }
  function applyPreferences() {
    document.dispatchEvent(
      new CustomEvent('editor:resize', { detail: (preferences.editor_fraction ?? 0.5) * 100 }),
    )
    document.dispatchEvent(new Event('proofing:changed'))
    if (!s.diagnostic)
      editor.commands.updateUser({
        name: preferences.profile['sender.name'] || tr('Vous'),
        color: '#286b91',
      })
    document.documentElement.dataset.appearance = preferences.appearance
    document.documentElement.style.setProperty(
      '--editor-font',
      preferences.font === 'sans' ? 'Inter' : preferences.font === 'mono' ? 'monospace' : 'Georgia',
    )
    document.documentElement.style.setProperty('--editor-size', `${preferences.size}px`)
  }
  function prefs() {
    preferencesPanel(
      showPanel(tr('Préférences')),
      catalogue,
      preferences,
      transport,
      async (next) => {
        await transport.invoke('preferences_save', { preferences: next })
        preferences = next
        applyPreferences()
        s.message(tr('Préférences enregistrées. Les documents existants conservent leurs données.'))
      },
      (error) => s.message(String(error), true),
    )
  }
  add('document.new', tr('Nouveau document'), () => confirmDiscard(create), 'Mod+N')
  add('document.open', tr('Ouvrir…'), () => confirmDiscard(() => guarded(() => open())), 'Mod+O')
  add('document.save', tr('Enregistrer'), () => guarded(() => saveFile()), 'Mod+S')
  add(
    'document.save_as',
    tr('Enregistrer sous…'),
    () => guarded(() => saveFile(true)),
    'Mod+Shift+S',
  )
  add('document.close', tr('Fermer le document'), () =>
    confirmDiscard(() => {
      if (s.isTauri) {
        guarded(async () => {
          await provider.flush()
          await transport.invoke('close_document', { revision: provider.revision })
        })
        return
      }
      setActive(false)
      dirty = false
    }),
  )
  if (s.isTauri)
    add(
      'app.quit',
      tr('Quitter'),
      () => confirmDiscard(() => guarded(() => transport.invoke('quit_app'))),
      'Mod+Q',
    )
  add(
    'document.export',
    tr('Exporter'),
    () =>
      guarded(async () => {
        await provider.flush()
        const folder = s.isTauri
          ? await transport.invoke<string | null>('choose_export_folder')
          : undefined
        if (s.isTauri && !folder) return
        const destination = await transport.invoke<string>('export_document', {
          destination: folder,
          snapshot: {
            revision: provider.revision,
            document: editor.getJSON(),
            settings: settingsFor(provider),
          },
        })
        s.message(`PDF et sources Typst exportés · ${destination}`)
      }),
    'Mod+E',
  )
  add('ui.preferences', tr('Préférences'), prefs, 'Mod+,')
  add('ui.palette', tr('Palette de commandes'), palette, 'Mod+Shift+P')
  add('ui.find', tr('Rechercher et remplacer'), search, 'Mod+F')
  add(
    'find.next',
    tr('Occurrence suivante'),
    () => {
      navigateSearch(editor)
    },
    'Mod+G',
  )
  add(
    'find.previous',
    tr('Occurrence précédente'),
    () => {
      navigateSearch(editor, true)
    },
    'Mod+Shift+G',
  )
  add('text.clear', tr('Effacer la mise en forme'), () => {
    editor.chain().focus().unsetAllMarks().run()
  })
  add('text.unlink', tr('Supprimer le lien'), () => {
    editor.chain().focus().unsetLink().run()
  })
  add('spelling.toggle', tr('Activer / désactiver la correction orthographique'), () =>
    guarded(async () => {
      preferences.spelling_enabled = preferences.spelling_enabled === false
      await transport.invoke('preferences_save', { preferences })
      applyPreferences()
    }),
  )
  add('spelling.next', tr('Erreur suivante'), () => {
    navigateSpelling(editor)
  })
  add('spelling.previous', tr('Erreur précédente'), () => {
    navigateSpelling(editor, true)
  })
  add('spelling.reset', tr('Réexaminer les mots ignorés'), () =>
    setSetting(provider, 'options.spelling_ignored', {}),
  )
  add('ui.information', tr('Paramètres du document'), () => {
    tab = tab ? null : 'general'
    renderMetadata()
  })
  document.addEventListener('editor:width', (event) => {
    const fraction = (event as CustomEvent<number>).detail / 100
    if (preferences.editor_fraction === fraction) return
    preferences.editor_fraction = fraction
    guarded(() => transport.invoke('preferences_save', { preferences }))
  })
  add('ui.peer', tr('Deuxième fenêtre'), () => guarded(() => transport.invoke('open_peer')))
  add('ui.connection', tr('Connexion / déconnexion'), () =>
    guarded(async () => {
      if (provider.online) await provider.disconnect()
      else await provider.reconnect()
    }),
  )
  for (const [title, ids] of [
    [
      tr('Fichier'),
      [
        'document.new',
        'document.open',
        'document.save',
        'document.save_as',
        'document.export',
        'document.close',
      ],
    ],
    [
      tr('Édition'),
      [
        'history.undo',
        'history.redo',
        'ui.find',
        'find.next',
        'find.previous',
        'text.clear',
        'text.unlink',
        'spelling.toggle',
        'ui.palette',
      ],
    ],
    [tr('Affichage'), ['ui.preferences', 'ui.peer', 'ui.connection']],
  ] as const) {
    const details = document.createElement('details')
    details.className = 'tool-menu'
    const summary = document.createElement('summary')
    summary.textContent = title
    summary.setAttribute('role', 'button')
    const entries = document.createElement('div')
    entries.className = 'tool-menu-content'
    for (const id of ids) {
      const command = commands.get(id)!
      const button = document.createElement('button')
      button.textContent = command.label
      button.dataset.command = id
      button.onclick = () => {
        details.open = false
        if (command.enabled?.() !== false) command.run()
      }
      entries.append(button)
    }
    details.append(summary, entries)
    menu.append(details)
  }
  for (const [label, name, action] of [
    [tr('Commandes'), 'pilcrow', palette],
    [tr('Rechercher'), 'search', search],
    [tr('Préférences'), 'settings-2', prefs],
  ] as const) {
    const button = document.createElement('button')
    button.innerHTML = icon(name)
    button.title = label
    button.setAttribute('aria-label', label)
    button.onclick = action
    side.append(button)
  }
  document.addEventListener(
    'keydown',
    (event) => {
      if (event.isComposing) return
      if (event.key === 'Escape') {
        if (overlay.open) overlay.close()
        else if (!panel.hidden) {
          panel.hidden = true
          searchState(editor, '')
          editor.commands.focus()
        } else {
          tab = null
          renderMetadata()
        }
        return
      }
      if (!(event.metaKey || event.ctrlKey)) return
      const key = `Mod+${event.shiftKey ? 'Shift+' : ''}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`
      for (const command of commands.values())
        if (command.shortcut === key && command.enabled?.() !== false) {
          if (
            (event.target instanceof HTMLInputElement ||
              event.target instanceof HTMLTextAreaElement) &&
            (['context.open', 'block.remove'].includes(command.id) ||
              command.id.startsWith('history.'))
          )
            return
          event.preventDefault()
          event.stopImmediatePropagation()
          command.run()
          return
        }
    },
    true,
  )
  $('create-document').onclick = () => confirmDiscard(create)
  $('open-document').onclick = () => guarded(() => open())
  const showRecovery = () => {
    if (!s.isTauri) {
      setActive(true)
      renderMetadata()
      return
    }
    overlay.replaceChildren()
    const title = document.createElement('h2')
    title.textContent = tr('Récupération après incident')
    const body = document.createElement('div')
    const close = document.createElement('button')
    close.textContent = tr('Annuler')
    close.onclick = () => overlay.close()
    const actions = document.createElement('div')
    actions.className = 'panel-actions'
    actions.append(close)
    overlay.append(title, body, actions)
    overlay.showModal()
    guarded(() =>
      recoveryPanel(
        body,
        transport,
        (id) =>
          confirmDiscard(() =>
            guarded(async () => {
              await provider.flush()
              await transport.invoke('recovery_restore', { id, revision: provider.revision })
            }),
          ),
        (error) => {
          const message = document.createElement('p')
          message.role = 'alert'
          message.textContent = String(error)
          body.append(message)
        },
      ),
    )
  }
  add('recovery.list', tr('Récupération après incident'), showRecovery)
  $('resume-document').onclick = showRecovery
  let recoveryTimer: ReturnType<typeof setTimeout>
  function scheduleRecovery() {
    if (!s.isTauri || !active) return
    clearTimeout(recoveryTimer)
    recoveryTimer = setTimeout(
      () =>
        void guarded(async () => {
          await provider.flush()
          await transport.invoke('recover', {
            snapshot: {
              revision: provider.revision,
              document: editor.getJSON(),
              settings: settingsFor(provider),
            },
            update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
          })
        }),
      2000,
    )
  }
  settings.observe(() => {
    dirty = true
    scheduleRecovery()
    renderMetadata()
    updateTitle()
    s.schedule()
  })
  editor.on('update', () => {
    scheduleRecovery()
    dirty = true
    updateTitle()
  })
  if (s.isTauri) {
    for (const [event, composing] of [
      ['compositionstart', true],
      ['compositionend', false],
    ] as const)
      document.addEventListener(
        event,
        () => {
          void transport.invoke('agent_input', { composing })
        },
        true,
      )
    if (!s.diagnostic)
      await getCurrentWindow().onCloseRequested((event) => {
        event.preventDefault()
        confirmDiscard(() =>
          guarded(async () => {
            clearTimeout(recoveryTimer)
            await transport.invoke('recovery_discard')
            await getCurrentWindow().destroy()
          }),
        )
      })
    await transport.listen('app:document', () => {
      clearTimeout(recoveryTimer)
      window.location.reload()
    })
    await transport.listen<string>('app:command', (id) => {
      const command = commands.get(id)
      if (command?.enabled?.() !== false) command?.run()
    })
  }
  applyPreferences()
  if (s.initialPath) guarded(() => open(s.initialPath!))
  mountAssistance(provider, transport, editor, showPanel, s.message, s.isTauri)
  mountProofing(
    editor,
    s.bands,
    provider,
    transport,
    () => ({
      personal: preferences.personal || {},
      spelling_enabled: preferences.spelling_enabled !== false,
    }),
    async (word, language) => {
      preferences.personal ||= {}
      preferences.personal[language] = [
        ...new Set([...(preferences.personal[language] || []), word]),
      ]
      await transport.invoke('preferences_save', { preferences })
    },
    s.message,
  )
  renderMetadata()
  setActive(active)
}

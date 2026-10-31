import { versionsPanel } from './versions-panel'
import { versionsAPI } from './versions-api'
import { metadataInputBinding } from './metadata-input'
import { recoveryError } from './recovery-errors'
import { workList, type WorkEntry } from './work-list'
import { recoveredPanel } from './recovered-panel'
import { reopenBand } from './form-recovery'
import {
  consumeRecovered,
  recoveredLabel,
  RECOVERED_CHANGED,
  type RecoveredItem,
} from './recovered-work'
import { privateDrafts } from './private-drafts'
import { decideClose, type CloseChoice } from './close-decision'
import { recoveryTransitions } from './recovery-transition'
import { stableJSON } from './provider'
import { privateRecovery } from './private-recovery'
import { historyFor } from './editing-history'
import { commandEditor } from './command-context'
import { interactionAvailability } from './command-availability'
import { commandAvailability, runCommand } from './public-commands'
import { mountCommandSurfaces } from './command-surfaces'
import { runFormatting } from './formatting'
import { documentContent } from './shared-inline-marks'
import { bibliographyPanel } from './bibliography-panel'
import { actionRow, keyboardScope, navigationGroup } from './keyboard-navigation'
import { commandPalette } from './command-palette'
import { searchPanel, navigateSearch } from './search-panel'
import { preferencesPanel, type Preferences } from './preferences-panel'
import { preferencesWriter } from './preferences-writer'
import { tr } from './i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import * as Y from 'yjs'
import type { Editor } from '@tiptap/core'
import type { Provider, Transport, Resource } from './provider'
import type { Capability, Registry } from './commands'
import { icon } from './commands'
import { mountAssistance } from './assistance'
import { mountProofing, searchState, navigateSpelling } from './proofing'
import type { BandHost } from './band-layout'
import { prepareImage, resourceCommand } from './resource-commands'
import { documentType, settingsFor, setSetting, setDocument, type Settings } from './state'
import { welcomeBrand } from './welcome-brand'
import type { StructuredMetadata } from './metadata-records'
import { metadataCollections } from './metadata-collections'
import { richZoneOwnsShortcut } from './rich-zone'
import {
  metadataFields,
  metadataSection,
  refreshMetadataFields,
  destroyMetadataFields,
} from './metadata-fields'
import { metadataCommands } from './metadata-commands'
import { documentOptions } from './document-options'
import { withCurrentSnapshot } from './snapshot-operation'
export interface ModelPolicy {
  outline: boolean
  citation_styles: string[]
}
export interface Model {
  id: string
  label: string
  description: string
  source: string
  fixedPage: { paper: string; landscape: boolean } | null
  policy?: ModelPolicy
}
export interface Field {
  field: string
  parameter: string
  label: string
  value: unknown
  group: string
  multiline: boolean
  kind: string
  required_for_export: boolean
}
export interface MetadataGroup {
  id: string
  label: string
  sections: {
    id: string
    label: string
    fields: string[]
    layout?: 'editorial' | 'compact' | 'inline'
  }[]
}
export interface DocumentType {
  structured?: StructuredMetadata | null
  capabilities: string[]
  id: string
  label: string
  canCreate: boolean
  settingsHeading: string
  metadata: Settings['metadata']
  fields: Field[]
  groups: MetadataGroup[]
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
  save: (includeDocumentEdits?: boolean) => Promise<void>
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
    dirty = !!provider.recoveredName,
    path: string | null = s.filePath || null,
    savedRevision = provider.revision
  let recovery: ReturnType<typeof privateRecovery> | undefined
  let transitions: Awaited<ReturnType<typeof recoveryTransitions>> | undefined
  let disposition: CloseChoice | null = null
  let versionPanel: ReturnType<typeof versionsPanel> | undefined
  let releaseRecoveredPanel: (() => void) | undefined
  const switchDocument = <T>(task: (revision: number) => Promise<T>) =>
    transitions ? transitions.run(task, disposition === 'discard') : task(provider.revision)
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
  const writePreferences = preferencesWriter(transport)
  async function persistPreferences(next: Preferences = preferences) {
    // Other producers (width, recent projects, dictionary) must see the new settings
    // while their disk write is pending, rather than composing a stale snapshot.
    preferences = next
    await writePreferences(next)
  }
  const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T
  document.querySelector('.badge')?.remove()
  $('rust-experiment').hidden = !s.diagnostic
  $('presence').hidden = !s.diagnostic
  $('synchronization').hidden = !active
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
  $<HTMLButtonElement>('resume-document').hidden = true
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
  navigationGroup(side, { axis: 'vertical' })
  const panel = document.createElement('aside')
  panel.className = 'side-panel'
  panel.hidden = true
  panel.innerHTML =
    '<div class="panel-title"><h2 id="side-title"></h2><button id="close-side" aria-label="Fermer le panneau">×</button></div><div id="side-content"></div>'
  workspace.append(panel)
  $('close-side').onclick = () => {
    versionPanel?.destroy()
    versionPanel = undefined
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
  let generalOptions: ReturnType<typeof documentOptions> | undefined
  let recordPanel:
    | { refresh: () => void; destroy: () => void; resume?: (item: RecoveredItem) => boolean }
    | undefined
  const metadataEditor = metadataCommands(editor, provider)
  const policies = new Map<string, ModelPolicy>(),
    loadingPolicies = new Set<string>()
  const currentPolicy = (): ModelPolicy | undefined => {
    const current = settingsFor(provider),
      kind = documentType(provider)
    const bundled = kind.models.find(
      (model) => model.id === current.modelId && model.source === current.template,
    )
    if (bundled?.policy) return bundled.policy
    if (!policies.has(current.template) && !loadingPolicies.has(current.template)) {
      loadingPolicies.add(current.template)
      void transport
        .invoke<ModelPolicy>('model_policy', { template: current.template })
        .then((policy) => {
          policies.set(current.template, policy)
          renderMetadata()
        })
        .catch(() => {})
    }
    return policies.get(current.template)
  }
  keyboardScope(panel, {
    close: () => {
      panel.hidden = true
      searchState(editor, '')
      editor.commands.focus()
    },
    priority: 10,
    tab: true,
  })
  keyboardScope(overlay, {
    close: () => overlay.close(),
    priority: 30,
    confirm: () => overlay.querySelector<HTMLButtonElement>('[data-default]')?.click(),
    tab: true,
  })
  keyboardScope(context, {
    close: () => {
      const previous = tab
      tab = null
      renderMetadata()
      document.getElementById(`metadata-tab-${previous}`)?.focus()
    },
    active: () => tab !== null,
    priority: 5,
    tab: true,
  })
  navigationGroup($('metadata-tabs'), { axis: 'horizontal', roving: true, activateOnMove: true })
  navigationGroup(menu, { axis: 'horizontal' })
  const guarded = (task: () => Promise<unknown> | void) => {
    try {
      const result = task()
      if (result instanceof Promise) void result.catch((e) => s.message(recoveryError(e), true))
    } catch (e) {
      s.message(recoveryError(e), true)
    }
  }
  const requiresDocument = new Set([
    'ui.versions',
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
    if (versionPanel) {
      versionPanel.destroy()
      versionPanel = undefined
      panel.hidden = true
    }
    active = value
    $('synchronization').hidden = !value
    welcome.hidden = value
    workspace.hidden = !value
    if (value) brand.finish()
    else brand.play()
    if (value) {
      editor.commands.focus()
      s.schedule()
    }
    updateTitle()
    document.dispatchEvent(new Event('commands:refresh'))
  }
  function updateTitle() {
    const title =
      path
        ?.split(/[\\/]/)
        .at(-1)
        ?.replace(/\.typsto$/, '') ||
      provider.recoveredName?.replace(/\.typsto$/, '') ||
      tr('Sans titre')
    if (s.isTauri)
      void transport.invoke('set_window_title', {
        title: active ? `${title}${dirty ? ' *' : ''}` : 'typstOgraphe',
      })
    header.dataset.document = title
  }
  function pendingInputs() {
    return (
      provider.recoveredItems.length +
      privateDrafts(provider)
        .snapshot()
        .filter((form) => !form.sourceId).length +
      ((provider.documentEdits?.draft?.() as unknown[] | undefined)?.length || 0) +
      (provider.rejected ? 1 : 0)
    )
  }
  function confirmDiscard(next: () => void, quitting = false) {
    guarded(async () => {
      const choice = await decideClose({
        dirty: active && (dirty || metadataEditor.dirty()),
        pending: active ? pendingInputs() : 0,
        otherWindows: quitting
          ? Math.max(0, (provider.synchronization?.windows.length || 1) - 1)
          : 0,
        save: () => saveFile(),
        keep: async () => {
          await recovery?.force()
        },
        discard: async () => {},
      })
      if (!choice) return
      disposition = choice
      if (choice !== 'discard') await recovery?.force()
      next()
    })
  }
  async function saveFile(as = false) {
    await recovery?.flush()
    await provider.flush(false)
    if (!s.isTauri) {
      await s.save(false)
      dirty = false
      return true
    }
    const destination =
      as || !path
        ? await transport.invoke<string | null>('choose_file', {
            save: true,
            defaultName: provider.recoveredName
              ? `${provider.recoveredName.replace(/\.typsto$/, '')} — ${tr('récupéré')}.typsto`
              : null,
          })
        : path
    if (!destination) return false
    const persistedRevision = await withCurrentSnapshot(
      provider,
      () => ({
        snapshot: {
          revision: provider.revision,
          document: documentContent(editor.getJSON()),
          settings: settingsFor(provider),
        },
        update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
      }),
      async (checkpoint) => {
        await transport.invoke('save_file', { path: destination, ...checkpoint })
        return checkpoint.snapshot.revision
      },
      false,
    )
    path = destination
    savedRevision = persistedRevision
    await remember(destination)
    dirty = provider.revision !== persistedRevision
    provider.recoveredName = null
    document.querySelector('.recovered-document-line')?.remove()
    updateTitle()
    s.message(tr('Document enregistré.'))
    return true
  }
  async function remember(file: string) {
    preferences.recent = [file, ...preferences.recent.filter((p) => p !== file)].slice(0, 12)
    await persistPreferences()
  }
  async function open(file?: string) {
    const selected = file || (await transport.invoke<string | null>('choose_file', { save: false }))
    if (!selected) return
    await recovery?.flush()
    await provider.flush(false)
    const opened = await switchDocument((revision) =>
      transport.invoke<{
        document: unknown
        settings: Settings
        resources: Resource[]
      }>('open_file', { path: selected, revision }),
    )
    if (s.isTauri || provider.nativeEditing) {
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
    historyFor(editor)?.clear()
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
    const controls = actionRow()
    const cancel = document.createElement('button')
    cancel.textContent = tr('Annuler')
    cancel.onclick = () => overlay.close()
    const submit = document.createElement('button')
    submit.className = 'primary'
    submit.dataset.default = ''
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
      if (s.isTauri || provider.nativeEditing) {
        if (t.id === 'courrier' && preferences.signature)
          metadata.fields.signature = preferences.signature.path
        guarded(async () => {
          await provider.flush(false)
          await recovery?.force()
          await switchDocument((revision) =>
            transport.invoke('new_document', {
              settings: { metadata, template: m.source, modelId: m.id },
              resources:
                t.id === 'courrier' && preferences.signature ? [preferences.signature] : [],
              revision,
            }),
          )
        })
        return
      }
      provider.doc.transact(() => {
        setDocument(provider, { metadata, template: m.source, modelId: m.id })
        editor.commands.clearContent()
      })
      if (t.id === 'courrier' && preferences.signature) {
        provider.addResource(preferences.signature)
        setSetting(provider, 'signature', preferences.signature.path)
      }
      path = null
      dirty = false
      historyFor(editor)?.clear()
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
    if (tab !== 'general' && !kind.groups.some((group) => group.id === tab)) tab = null
    $('document-type-label').textContent = kind.settingsHeading
    const tabs = $('metadata-tabs')
    const groups = [{ id: 'general', label: tr('Général') }, ...kind.groups]
    if (
      tabs.children.length !== groups.length ||
      groups.some((group, index) => tabs.children[index]?.id !== `metadata-tab-${group.id}`)
    )
      tabs.replaceChildren()
    for (const group of groups) {
      const button =
        tabs.querySelector<HTMLButtonElement>(`#metadata-tab-${group.id}`) ||
        document.createElement('button')
      button.textContent = group.label
      button.id = `metadata-tab-${group.id}`
      button.setAttribute('aria-pressed', String(tab === group.id))
      button.onclick = () => {
        tab = tab === group.id ? null : group.id
        renderMetadata()
      }
      if (!button.parentElement) tabs.append(button)
    }
    const content = $('metadata-content')
    content.hidden = tab === null
    const identity = String(settings.get('identity') || '')
    const samePanel =
      content.dataset.documentType === kind.id &&
      content.dataset.tab === tab &&
      content.dataset.identity === identity
    if (samePanel && tab === 'general' && generalOptions) {
      generalOptions.update(current)
      return
    }
    if (samePanel && recordPanel) {
      recordPanel.refresh()
      return
    }
    if (samePanel && content.contains(document.activeElement)) {
      refreshMetadataFields(content, current.metadata.fields)
      return
    }
    generalOptions?.destroy()
    generalOptions = undefined
    recordPanel?.destroy()
    recordPanel = undefined
    destroyMetadataFields(content)
    content.dataset.identity = identity
    content.dataset.documentType = kind.id
    content.dataset.tab = tab || ''
    content.replaceChildren()
    if (!tab) return
    if (tab === 'general') {
      generalOptions = documentOptions(
        content,
        kind,
        catalogue,
        current,
        { commands: metadataEditor, read: () => settingsFor(provider) },
        currentPolicy,
      )
    } else if (kind.structured?.bibliography?.tab === tab) {
      recordPanel = bibliographyPanel(content, provider, editor, transport, kind, currentPolicy)
    } else if (kind.structured?.panels?.some((panel) => panel.id === tab)) {
      recordPanel = metadataCollections(
        content,
        provider,
        kind.structured,
        kind.structured.panels.find((panel) => panel.id === tab)!,
        catalogue,
        editor,
        () =>
          String(
            settings.get('options.language') ??
              (settings.get('document') as Settings).metadata.fields.options.language,
          ),
      )
    } else {
      const group = kind.groups.find((group) => group.id === tab)
      if (group)
        metadataFields(
          content,
          group,
          kind.fields,
          current.metadata.fields,
          (field, value) =>
            metadataEditor.set(
              { kind: 'metadata', field },
              value,
              () =>
                settingsFor(provider).metadata.fields[
                  kind.fields.find((item) => item.field === field)!.parameter
                ],
            ),
          {
            commands: metadataEditor,
            read: (field) => settingsFor(provider).metadata.fields[field.parameter],
          },
        )
    }
    if (tab === 'sender') {
      const actions = actionRow()
      for (const [label, useDefault] of [
        [tr('Sans signature'), false],
        [tr('Signature des préférences'), true],
      ] as const) {
        const button = document.createElement('button')
        button.textContent = label
        button.onclick = () => {
          const value = useDefault ? preferences.signature?.path || null : null
          const before = settingsFor(provider).metadata.fields.signature
          guarded(() =>
            metadataEditor.run(
              [
                ...(value && preferences.signature ? [resourceCommand(preferences.signature)] : []),
                { id: 'document.set_signature', params: { path: value } },
              ],
              () => settingsFor(provider).metadata.fields.signature === before,
            ),
          )
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
          const before = settingsFor(provider).metadata.fields.signature
          const resource = await prepareImage(provider, transport, file)
          await metadataEditor.run(
            [
              resourceCommand(resource),
              { id: 'document.set_signature', params: { path: resource.path } },
            ],
            () => settingsFor(provider).metadata.fields.signature === before,
          )
          image.value = ''
        })
      const signature = metadataSection({
        id: 'signature',
        label: tr('Signature du document'),
        layout: 'editorial',
      })
      signature.root.classList.add('metadata-signature')
      signature.content.classList.add('metadata-signature-controls')
      signature.content.append(image, actions)
      content.append(signature.root)
    }
  }
  function showPanel(title: string) {
    versionPanel?.destroy()
    versionPanel = undefined
    releaseRecoveredPanel?.()
    releaseRecoveredPanel = undefined
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
      runCommand(command)
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
      provider.presence.setUser({
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
        await persistPreferences(next)
        applyPreferences()
        s.message(tr('Préférences enregistrées. Les documents existants conservent leurs données.'))
      },
      (error) => s.message(recoveryError(error), true),
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
      if (s.isTauri || provider.nativeEditing) {
        guarded(async () => {
          await switchDocument((revision) => transport.invoke('close_document', { revision }))
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
      () =>
        confirmDiscard(
          () =>
            guarded(async () => {
              await switchDocument(async () => {
                if (disposition === 'discard')
                  await transport.invoke('work_abandon', { generation: provider.generation })
                await transport.invoke('quit_app')
              })
            }),
          true,
        ),
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
        const destination = await withCurrentSnapshot(
          provider,
          () => ({
            revision: provider.revision,
            document: documentContent(editor.getJSON()),
            settings: settingsFor(provider),
          }),
          (snapshot) =>
            transport.invoke<string>('export_document', { destination: folder, snapshot }),
        )
        s.message(`PDF et sources Typst exportés · ${destination}`)
      }),
    'Mod+E',
  )
  provider.onSaved = (result) => {
    if (result.document !== provider.generation || result.revision < savedRevision) return
    const duplicate = path === result.path && savedRevision === result.revision && !dirty
    path = result.path
    savedRevision = result.revision
    dirty = provider.revision !== result.revision
    updateTitle()
    if (!duplicate) {
      void remember(result.path).catch((error) => s.message(recoveryError(error), true))
      if (result.warning) s.message(result.warning, true)
      else s.message(tr('Document enregistré.'))
    }
    void versionPanel?.refresh()
  }
  add('ui.versions', tr('Versions'), () => {
    const api = versionsAPI({
      creator: preferences.profile['sender.name'] || null,
      provider,
      transport,
      destination: async () =>
        path ? undefined : transport.invoke<string | null>('choose_file', { save: true }),
      saved: async (result) => {
        path = result.path
        savedRevision = result.revision
        dirty = provider.revision !== result.revision
        provider.recoveredName = null
        updateTitle()
        try {
          await remember(result.path)
        } catch (error) {
          s.message(recoveryError(error), true)
          return
        }
        if (result.warning) s.message(result.warning, true)
        else s.message(tr('Versions enregistrées.'))
      },
    })
    versionPanel = versionsPanel(showPanel(tr('Versions')), api, catalogue, (error) =>
      s.message(recoveryError(error), true),
    )
  })
  void transport.listen<{ document: string; path: string; revision: number }>(
    'web-editor:versions',
    (event) => {
      if (event.document !== provider.generation) return
      path = event.path
      savedRevision = event.revision
      dirty = provider.revision !== event.revision
      updateTitle()
      void versionPanel?.refresh()
    },
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
    runFormatting(commandEditor(editor), 'text.clear')
  })
  add('text.unlink', tr('Supprimer le lien'), () => {
    runFormatting(commandEditor(editor), 'text.remove_link')
  })
  for (const [id, policy] of [
    ['text.clear', 'style'],
    ['text.unlink', 'link'],
  ] as const) {
    const original = commands.get(id)!
    commands.set(id, {
      ...original,
      availability: () => interactionAvailability(commandEditor(editor), policy),
    })
  }
  add('spelling.toggle', tr('Activer / désactiver la correction orthographique'), () =>
    guarded(async () => {
      preferences.spelling_enabled = preferences.spelling_enabled === false
      await persistPreferences()
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
    metadataEditor.set(
      { kind: 'option', name: 'spelling_ignored' },
      {},
      () => settingsFor(provider).metadata.fields.options.spelling_ignored,
    ),
  )
  add('ui.information', tr('Paramètres du document'), () => {
    tab = tab ? null : 'general'
    renderMetadata()
  })
  document.addEventListener('editor:width', (event) => {
    const fraction = (event as CustomEvent<number>).detail / 100
    if (preferences.editor_fraction === fraction) return
    preferences.editor_fraction = fraction
    guarded(() => persistPreferences())
  })
  for (const [id, enabled, label] of [
    ['synchronization.enable', true, tr('Activer la synchronisation')],
    ['synchronization.disable', false, tr('Arrêter la synchronisation')],
  ] as const) {
    requiresDocument.add(id)
    commands.set(id, {
      id,
      label,
      enabled: () =>
        active &&
        provider.online &&
        !provider.rejected &&
        !provider.synchronizationBusy &&
        !!provider.synchronization &&
        provider.synchronization.enabled !== enabled &&
        (enabled || provider.synchronization.windows.length < 2),
      run: () =>
        guarded(async () => {
          try {
            await provider.setSynchronization(enabled)
          } catch (error) {
            if (String(error) === 'synchronization_other_windows')
              throw new Error(tr('Fermez les autres fenêtres avant d’arrêter la synchronisation.'))
            throw error
          }
        }),
    })
  }
  add('ui.peer', tr('Deuxième fenêtre'), () => guarded(() => transport.invoke('open_peer')))
  add('ui.connection', tr('Connexion / déconnexion'), () =>
    guarded(async () => {
      if (provider.online) await provider.disconnect()
      else await provider.reconnect()
    }),
  )
  const commandSurfaces = mountCommandSurfaces({
    editor,
    commands,
    capabilities: catalogue.entries,
    requiresDocument,
    active: () => active,
    native: s.isTauri ? transport : undefined,
    onError: (error) => s.message(recoveryError(error), true),
  })
  window.addEventListener('pagehide', () => commandSurfaces.destroy(), { once: true })
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
    [
      tr('Affichage'),
      [
        'ui.versions',
        'ui.preferences',
        'ui.peer',
        'synchronization.enable',
        'synchronization.disable',
        'ui.connection',
      ],
    ],
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
        runCommand(command)
      }
      entries.append(button)
    }
    details.append(summary, entries)
    menu.append(details)
  }
  for (const [label, name, action] of [
    [tr('Commandes'), 'pilcrow', 'ui.palette'],
    [tr('Rechercher'), 'search', 'ui.find'],
    [tr('Versions'), 'history', 'ui.versions'],
    [tr('Préférences'), 'settings-2', 'ui.preferences'],
  ] as const) {
    const button = document.createElement('button')
    button.innerHTML = icon(name)
    button.title = label
    button.setAttribute('aria-label', label)
    button.dataset.command = action
    button.onclick = () => {
      runCommand(commands.get(action)!)
    }
    side.append(button)
  }
  document.addEventListener(
    'keydown',
    (event) => {
      if (document.querySelector('dialog[open]')) return
      if (event.isComposing) return
      if (!(event.metaKey || event.ctrlKey)) return
      const key = `Mod+${event.shiftKey ? 'Shift+' : ''}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`
      for (const command of commands.values())
        if (command.shortcut === key && commandAvailability(command).enabled) {
          // Named rich zones own their text shortcuts and share the same undo owner.
          // Let their editor receive the key instead of focusing the document body.
          if (richZoneOwnsShortcut(event.target, command.id)) return
          if (
            (event.target instanceof HTMLInputElement ||
              event.target instanceof HTMLTextAreaElement) &&
            (['context.open', 'block.remove'].includes(command.id) ||
              command.id.startsWith('history.'))
          )
            return
          event.preventDefault()
          event.stopImmediatePropagation()
          runCommand(command)
          return
        }
    },
    true,
  )
  $('create-document').onclick = () => confirmDiscard(create)
  $('open-document').onclick = () => confirmDiscard(() => guarded(() => open()))
  const resumeWork = (entry: WorkEntry) =>
    confirmDiscard(() =>
      guarded(() =>
        switchDocument((revision) => transport.invoke('work_restore', { id: entry.id, revision })),
      ),
    )
  const recoveryHome = document.createElement('section')
  recoveryHome.className = 'recovery-home'
  const recoveryHeading = document.createElement('h2')
  recoveryHeading.textContent = tr('Travail à reprendre')
  const recoveryList = document.createElement('div')
  recoveryHome.append(recoveryHeading, recoveryList)
  welcome.querySelector('.welcome-card')!.append(recoveryHome)
  const refreshRecovery = async () => {
    if (!provider.nativeEditing) return
    await workList(recoveryList, transport, resumeWork, (error) =>
      s.message(recoveryError(error), true),
    )
    recoveryHome.hidden = recoveryList.hidden
  }
  const showRecovery = () =>
    guarded(async () => {
      if (active)
        await workList(showPanel(tr('Travail à reprendre')), transport, resumeWork, (error) =>
          s.message(recoveryError(error), true),
        )
      else await refreshRecovery()
    })
  add('recovery.list', tr('Travail à reprendre'), showRecovery)
  $('resume-document').onclick = showRecovery
  const reopenRecovered = (item: RecoveredItem) => {
    const kind = documentType(provider)
    if (item.kind === 'form') {
      const context = item.payload.context
      if (context?.kind === 'record') {
        const definition = kind.structured?.panels?.find(
          (panel) => panel.collection === context.collection,
        )
        if (!definition) return false
        tab = definition.id
        renderMetadata()
        return recordPanel?.resume?.(item) || false
      }
      if (context?.kind === 'band') {
        if (context.root?.kind === 'record') {
          const definition = kind.structured?.panels?.find(
            (panel) => panel.collection === context.root.collection,
          )
          if (definition) {
            tab = definition.id
            renderMetadata()
          }
        }
        return reopenBand(provider, item)
      }
    }
    if (item.kind === 'field') {
      const { target, value, base } = item.payload
      let read: (() => unknown) | undefined
      if (target?.kind === 'metadata') {
        const field = kind.fields.find((field) => field.field === target.field)
        if (!field) return false
        read = () => settingsFor(provider).metadata.fields[field.parameter]
        tab =
          kind.groups.find((group) =>
            group.sections.some((section) => section.fields.includes(target.field)),
          )?.id || field.group
      } else if (target?.kind === 'option') {
        read = () => settingsFor(provider).metadata.fields.options[target.name]
        tab = 'general'
      } else if (target?.kind === 'model') {
        read = () => ({
          modelId: settingsFor(provider).modelId,
          template: settingsFor(provider).template,
        })
        tab = 'general'
      }
      if (!read || stableJSON(read()) !== stableJSON(base)) return false
      renderMetadata()
      const input = [
        ...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
          '#metadata-content input, #metadata-content textarea, #metadata-content select',
        ),
      ].find(
        (control) =>
          !control.disabled &&
          !control.closest('[hidden]') &&
          stableJSON(metadataInputBinding(control)?.target) === stableJSON(target),
      )
      if (!input) return false
      metadataEditor.restore(
        target,
        value,
        read,
        () => consumeRecovered(provider, item.id),
        item.id,
      )
      input.focus()
      return true
    }
    return false
  }
  function showRecoveredItems() {
    releaseRecoveredPanel?.()
    releaseRecoveredPanel = recoveredPanel(
      showPanel(tr('Saisies à reprendre')),
      provider,
      reopenRecovered,
      s.isTauri,
      (item) => {
        const kind = documentType(provider),
          target = item.payload.target
        if (item.kind === 'field' && target?.kind === 'metadata')
          return kind.fields.find((f) => f.field === target.field)?.label || recoveredLabel(item)
        const context = item.payload.context
        if (item.kind === 'form' && context?.kind === 'record')
          return (
            kind.structured?.collections.find((c) => c.field === context.collection)?.itemLabel ||
            recoveredLabel(item)
          )
        return recoveredLabel(item)
      },
    )
  }
  add('ui.recovered', tr('Saisies à reprendre'), showRecoveredItems)
  requiresDocument.add('ui.recovered')
  const recoveredButton = document.createElement('button')
  recoveredButton.innerHTML = icon('history')
  recoveredButton.setAttribute('aria-label', tr('Saisies à reprendre'))
  recoveredButton.onclick = showRecoveredItems
  side.prepend(recoveredButton)
  if (provider.recoveredName) {
    const notice = document.createElement('div')
    notice.className = 'recovered-document-line'
    notice.textContent = tr('Document récupéré · copie indépendante')
    context.before(notice)
  }
  const importWork = () =>
    guarded(async () => {
      const selected = await transport.invoke<string | null>('choose_work_file', { save: false })
      if (selected)
        confirmDiscard(() =>
          guarded(() =>
            switchDocument((revision) =>
              transport.invoke('work_import', { path: selected, revision }),
            ),
          ),
        )
    })
  add('recovery.import', tr('Ouvrir un brouillon de secours…'), importWork)
  for (const id of ['recovery.list', 'recovery.import']) {
    const button = document.createElement('button')
    button.textContent = commands.get(id)!.label
    button.dataset.command = id
    button.onclick = () => {
      menu.querySelectorAll('details').forEach((item) => (item.open = false))
      runCommand(commands.get(id)!)
    }
    menu.querySelector('.tool-menu-content')?.append(button)
  }
  const importButton = document.createElement('button')
  importButton.textContent = tr('Ouvrir un brouillon de secours…')
  importButton.onclick = importWork
  recoveryHome.append(importButton)
  recovery = provider.nativeEditing
    ? privateRecovery(
        editor,
        provider,
        () => active,
        (error) => s.message(recoveryError(error), true),
      )
    : undefined
  const scheduleRecovery = () => recovery?.changed()
  if (provider.nativeEditing) transitions = await recoveryTransitions(provider, transport)
  if (active && !path) scheduleRecovery()
  document.addEventListener(RECOVERED_CHANGED, (event) => {
    if ((event as CustomEvent).detail === provider)
      recoveredButton.hidden = !provider.recoveredItems.length
  })
  recoveredButton.hidden = !provider.recoveredItems.length
  await refreshRecovery()
  if (provider.recoveredItems.length) showRecoveredItems()
  let metadataRefreshQueued = false
  document.addEventListener('metadata:draft', () => {
    scheduleRecovery()
    if (metadataEditor.dirty()) {
      dirty = true
      updateTitle()
    }
  })
  settings.observeDeep(() => {
    dirty = true
    scheduleRecovery()
    if (!metadataRefreshQueued) {
      metadataRefreshQueued = true
      queueMicrotask(() => {
        metadataRefreshQueued = false
        renderMetadata()
      })
    }
    updateTitle()
    s.schedule()
  })
  editor.on('update', () => {
    scheduleRecovery()
    dirty = true
    updateTitle()
  })
  const closeView = () =>
    confirmDiscard(() =>
      guarded(async () => {
        if (s.isTauri) {
          if (disposition === 'discard')
            await transport.invoke('work_abandon', { generation: provider.generation })
          else await recovery?.force()
          await getCurrentWindow().destroy()
        } else await switchDocument((revision) => transport.invoke('close_document', { revision }))
      }),
    )
  await transport.listen<{ window: string }>('app:request-close', (value) => {
    if (value.window === provider.id) closeView()
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
        closeView()
      })
    await transport.listen<string>('app:command', (id) => {
      if (document.querySelector('dialog[open]')) return
      const command = commands.get(id)
      if (command) runCommand(command)
    })
  }
  await transport.listen('app:document', () => {
    recovery?.stop()
    window.location.reload()
  })
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
      await persistPreferences()
    },
    s.message,
  )
  renderMetadata()
  setActive(active)
}

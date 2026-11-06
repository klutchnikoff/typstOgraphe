import { inlineContentTarget } from './inline-objects'
import { crossReferencePanel } from './cross-reference-panel'
import { setResolvedReferences } from './cross-reference-index'
import { registerRecoveryOpener } from './form-recovery'
import { refreshCommandButton } from './public-commands'
import { draftRecovery } from './draft-recovery'
import { historyFor } from './editing-history'
import { documentContent } from './shared-inline-marks'
import { initializeUI, tr } from './i18n'
import { type Editor, getSchema } from '@tiptap/core'
import { EditorState } from '@tiptap/pm/state'
import { normalizeWhitespace } from './document-whitespace'
import { prosemirrorJSONToYDoc, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { createRichZone, richZoneExtensions } from './rich-zone'
import { Provider, stableJSON, ACCEPTED_PROPOSAL, type Resource, type Transport } from './provider'
import { createRegistry, toolbar, refreshCommands, type Registry } from './commands'
import { widgetTarget, removeWidget } from './widgets'
import { openCorrection, correctionAtCursor } from './proofing'
import { widgetPresentations } from './widget-frame'
import { hasSettings, settingsForNode } from './capability-settings'
import { mountShell, type Catalogue } from './shell'
import { documentType, initializeSettings, settingsFor } from './state'
import { previewController } from './preview'
import { renderQueue } from './render-queue'
import { uiRefresh } from './ui-refresh'
import { documentStatistics } from './document-statistics'
import { withCurrentSnapshot } from './snapshot-operation'
import { resizableSeparator } from './resizable-separator'
import { installMenus } from './menus'
import { BandHost } from './band-layout'
import { propertyPanel } from './properties'
import { tablePanel } from './table-panel'
import { planBlockInsertion } from './editing'
import { applyObjectEdit, widgetBatch } from './object-commands'
import { prepareImage, resourceCommand } from './resource-commands'
import { inlineObjectTarget } from './inline-objects'
import { activeInlineObject } from './inline-object-focus'
import { citationPanel } from './citation-panel'
import { referenceRecords, setCitationEntries, type ReferenceEntry } from './citation-index'
import './style.css'
import './versions.css'
import { errorText } from './command-error'

declare global {
  interface Window {
    __SPIKE_TRANSPORT__?: { id: string; transport: Transport }
    __spike?: { editor: Editor; provider: Provider; save: () => Promise<void>; ready: boolean }
  }
}

const recipeBuild = import.meta.env.MODE === 'recipe' || import.meta.env.MODE === 'browser-test'
const app = document.querySelector<HTMLDivElement>('#app')!
installMenus()
app.innerHTML = `
  <header><div class="identity"><strong>typstOgraphe</strong></div>
    <div class="session-tools"><button id="peer">Deuxième fenêtre</button><button id="connection" aria-pressed="true">Connecté</button><button id="save">Enregistrer</button><button id="export">Exporter</button></div></header>
  <main><section class="writing" aria-label="Éditeur du document">
    <nav id="toolbar" aria-label="Mise en forme"></nav>
    <div class="document-info"><span>Courrier personnel</span><span>Français · A4 · Modèle local</span></div>
    <div id="editor-scroll"><div id="editor"></div></div>
  </section><div class="splitter" role="separator" tabindex="0" aria-label="Largeur des colonnes" aria-orientation="vertical" aria-valuemin="10" aria-valuemax="90" aria-valuenow="50"></div>
  <section class="preview" aria-label="Aperçu"><div class="preview-title"><strong id="preview-heading" role="status">Aperçu</strong><span id="render-status" role="status">Préparation…</span></div><div id="pages"><p class="empty">Le rendu du document apparaîtra ici.</p></div></section></main>
  <footer><span id="statistics">Chargement…</span><span id="presence"></span><button id="synchronization" type="button" aria-pressed="false"></button><span id="message" role="status"></span></footer>
  <input id="image-file" type="file" accept="image/png,image/jpeg,image/svg+xml" hidden>
`
const element = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T
const message = (text: string, error = false) => {
  element('message').textContent = text
  element('message').classList.toggle('error', error)
}
const isTauri = '__TAURI_INTERNALS__' in window
const browserHost =
  !isTauri && import.meta.env.MODE === 'browser-test'
    ? await (await import('./browser-transport')).browserTransport()
    : undefined
const host = (recipeBuild ? window.__SPIKE_TRANSPORT__ : undefined) ??
  browserHost ?? {
    id: isTauri ? getCurrentWebviewWindow().label : 'browser',
    transport: {
      invoke: <T>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args),
      listen: async <T>(name: string, handler: (payload: T) => void) =>
        listen<T>(name, (event) => handler(event.payload)),
    } satisfies Transport,
  }
const diagnosticMode =
  recipeBuild && isTauri ? await host.transport.invoke<string | null>('diagnostic_mode') : null
const synchronizationMeter = diagnosticMode?.startsWith('synchronization')
  ? (await import('./synchronization-performance')).measureTransport(host.transport)
  : undefined
if (synchronizationMeter) host.transport = synchronizationMeter.transport
const provider = new Provider(host.id, host.transport)
let editor: Editor
let properties: ReturnType<typeof propertyPanel> | undefined
let citations: ReturnType<typeof citationPanel> | undefined
let tables: ReturnType<typeof tablePanel> | undefined
let crossReferences: ReturnType<typeof crossReferencePanel> | undefined
const rendering = renderQueue(render)
const interfaceRefresh = uiRefresh(updateInterface)
let onPreview: ((revision: number) => void) | undefined
let restoringError = false
let stopping = false
const statistics = documentStatistics(host.transport, element('statistics'))
let version = 0
let preview: ReturnType<typeof previewController> | undefined

function propertyTarget(): { pos: number; kind: string } | undefined {
  const { from } = editor.state.selection
  const inline = activeInlineObject(editor.state)
  const widget = widgetTarget(editor)
  const content = inlineContentTarget(editor)
  const target = inline
    ? { pos: inline.pos, kind: inline.node.type.name }
    : content
      ? { pos: content.pos, kind: content.node.type.name }
      : editor.isActive('link')
        ? { pos: from, kind: 'link' }
        : widget
          ? { pos: widget.pos, kind: widget.node.type.name }
          : undefined
  return target && hasSettings(settingsForNode(target.kind, contentCatalogue.entries))
    ? target
    : undefined
}
function openProperties(pos: number, kind: string, remove = false) {
  if (kind === 'tableFigure' && !remove) tables?.open(pos)
  else if (kind === 'crossReference' && !remove) crossReferences?.open(pos)
  else if (kind === 'citation' && !remove) citations?.open(pos)
  else properties?.open(pos, kind, remove)
}

let commands: Registry = new Map()
let contentCatalogue: Catalogue

function refresh() {
  interfaceRefresh.request()
}
function updateInterface() {
  if (!editor || editor.isDestroyed || stopping) return
  refreshCommands(commands)
  tables?.refresh()
  statistics.refresh(editor)
  refreshSynchronization()
  const connection = element<HTMLButtonElement>('connection')
  connection.textContent = provider.online ? tr('Connecté') : tr('Hors connexion')
  connection.setAttribute('aria-pressed', String(provider.online))
}
function refreshSynchronization() {
  const people = [...(provider.awareness?.getStates().values() || [])]
    .map((s) => s.user?.name)
    .filter(Boolean)
  element('presence').textContent = people.join(' · ')
  const button = element<HTMLButtonElement>('synchronization')
  const enabled = !!provider.synchronization?.enabled
  button.textContent = provider.synchronizationBusy
    ? tr('Activation après la saisie…')
    : enabled
      ? tr('Synchronisation active')
      : tr('Activer la synchronisation')
  button.setAttribute('aria-pressed', String(enabled))
  const command = commands.get(enabled ? 'synchronization.disable' : 'synchronization.enable')
  if (command) refreshCommandButton(button, command)
  if (enabled && provider.synchronization!.windows.length > 1)
    button.title = tr('Fermez les autres fenêtres avant d’arrêter la synchronisation.')
}
function schedule() {
  if (stopping) return
  version++
  preview?.invalidate()
  element('render-status').hidden = false
  if (!provider.online) {
    element('render-status').textContent = tr('Modifications locales · hors connexion')
    return
  }
  element('render-status').textContent = tr('Mise à jour…')
  rendering.request()
}
async function render() {
  if (!editor || !preview || !provider.online || provider.composing || restoringError) return
  const localVersion = version
  try {
    await provider.flush()
    if (localVersion !== version) return
    const { result } = await preview.request(snapshot(), localVersion)
    if (localVersion !== version) return
    if (!(await preview?.show(result, localVersion)) || localVersion !== version) return
    setResolvedReferences(editor, result.references || [])
    const count = result.pageSizes?.length ?? result.pages.length
    element('preview-heading').textContent = tr(
      count === 1 ? 'Aperçu ({count} page)' : 'Aperçu ({count} pages)',
      { count },
    )
    element('render-status').textContent = ''
    element('render-status').hidden = true
    element('render-status').classList.remove('error')
    onPreview?.(result.revision)
  } catch (error) {
    if (String(error) === 'preview-reset') {
      preview?.reset()
      rendering.request()
      return
    }
    if (String(error) === 'stale') {
      rendering.request()
      return
    }
    element('render-status').hidden = false
    element('render-status').textContent = tr('Contenu à corriger · dernier rendu conservé')
    element('render-status').classList.add('error')
    message(errorText(error), true)
  }
}
function snapshot() {
  return {
    revision: provider.revision,
    document: documentContent(editor.getJSON()),
    settings: settingsFor(provider),
  }
}
async function save(includeDocumentEdits = true) {
  if (!provider.online || restoringError)
    throw new Error(tr('Reconnectez cette fenêtre avant d’enregistrer.'))
  const path = await withCurrentSnapshot(
    provider,
    () => ({ snapshot: snapshot(), update: Array.from(Y.encodeStateAsUpdate(provider.doc)) }),
    (checkpoint) => host.transport.invoke<string>('save', checkpoint),
    includeDocumentEdits,
  )
  message(`${tr('Session enregistrée')} · ${path}`)
}
function action(task: () => Promise<unknown>) {
  void task().catch((error) => message(errorText(error), true))
}
element('save').addEventListener('click', () => action(save))
element('peer').addEventListener('click', () => action(() => host.transport.invoke('open_peer')))
element('export').addEventListener('click', () =>
  action(async () => {
    if (!provider.online) throw new Error(tr('Reconnectez cette fenêtre avant d’exporter.'))
    const path = await withCurrentSnapshot(provider, snapshot, (snapshot) =>
      host.transport.invoke<string>('export_document', { snapshot }),
    )
    message(`${tr('PDF et sources Typst exportés')} · ${path}`)
  }),
)
element('synchronization').addEventListener('click', () => {
  commands
    .get(provider.synchronization?.enabled ? 'synchronization.disable' : 'synchronization.enable')
    ?.run()
})
element('synchronization').addEventListener('mousedown', (event) => event.preventDefault())
element('connection').addEventListener('click', () =>
  action(async () => {
    if (provider.online) await provider.disconnect()
    else await provider.reconnect()
    refresh()
    schedule()
  }),
)
element<HTMLInputElement>('image-file').addEventListener('change', (event) =>
  action(async () => {
    const file = (event.target as HTMLInputElement).files?.[0]
    if (!file) return
    const document = editor.state.doc,
      selection = editor.state.selection,
      replacement = properties?.imageReplacement()
    const resource = await prepareImage(provider, host.transport, file)
    if (!document.eq(editor.state.doc) || !selection.eq(editor.state.selection))
      throw new Error(tr('Le document a changé. Réessayez.'))
    if (replacement) {
      replacement(resource)
      element<HTMLInputElement>('image-file').value = ''
      return
    }
    const transaction = planBlockInsertion(editor, {
      type: 'figure',
      attrs: { src: resource.path, alt: '' },
      content: [],
    })
    if (transaction)
      await applyObjectEdit(editor, transaction, [
        resourceCommand(resource),
        ...widgetBatch(editor, transaction),
      ])
    element<HTMLInputElement>('image-file').value = ''
  }),
)
document.addEventListener('keydown', (event) => {
  if (event.defaultPrevented || event.isComposing || document.querySelector('dialog[open]')) return
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    action(save)
  }
})

const splitter = document.querySelector<HTMLElement>('.splitter')!
function resize(fraction: number, persist = true) {
  const value = Math.max(10, Math.min(90, fraction))
  document.documentElement.style.setProperty('--editor-width', `${value}%`)
  splitter.setAttribute('aria-valuenow', String(Math.round(value)))
  if (persist) document.dispatchEvent(new CustomEvent('editor:width', { detail: value }))
}
document.addEventListener('editor:resize', (event) =>
  resize((event as CustomEvent<number>).detail, false),
)
resizableSeparator(splitter, {
  value: () => Number(splitter.getAttribute('aria-valuenow')),
  bounds: () => ({ min: 10, max: 90 }),
  pointer: (event) => {
    const workspace = document.querySelector('main')!
    const box = workspace.getBoundingClientRect()
    const available = box.width - parseFloat(getComputedStyle(workspace).paddingRight || '0')
    return ((event.clientX - box.left) / available) * 100
  },
  change: resize,
  step: 2,
  defaultValue: 50,
})

async function start() {
  if (!isTauri && !(recipeBuild && window.__SPIKE_TRANSPORT__) && !browserHost)
    throw new Error(
      'Lancez l’application avec « npm start ». Le navigateur seul ne fournit pas le cœur Rust.',
    )
  contentCatalogue = await host.transport.invoke<Catalogue>('content_catalogue')
  initializeUI(contentCatalogue.messages)
  document.documentElement.lang = contentCatalogue.locale
  for (const [id, label] of [
    ['peer', 'Deuxième fenêtre'],
    ['connection', 'Connecté'],
    ['save', 'Enregistrer'],
    ['export', 'Exporter'],
  ])
    element(id).textContent = tr(label)
  element('preview-heading').textContent = tr('Aperçu')
  document.querySelector('.preview')!.setAttribute('aria-label', tr('Aperçu'))
  const presentations = widgetPresentations(contentCatalogue.entries)
  const schema = getSchema(
    richZoneExtensions('document', { imageURL: (path) => provider.imageURL(path), presentations }),
  )
  const bootstrap = await provider.start(
    (fixture) => {
      const seed = prosemirrorJSONToYDoc(schema, fixture, 'default')
      const update = Y.encodeStateAsUpdate(seed)
      seed.destroy()
      return update
    },
    (bootstrap) => initializeSettings(provider, contentCatalogue, bootstrap.settings),
  )
  editor = createRichZone({
    profile: 'document',
    element: element('editor'),
    label: tr('Corps du document'),
    imageURL: (path) => provider.imageURL(path),
    properties: openProperties,
    presentations,
    binding: { kind: 'document', document: provider.doc, presence: provider },
    openLink: (zone) => openProperties(zone.state.selection.from, 'link'),
    openContext: () => openContext(),
    onTransaction: ({ transaction }) => {
      refresh()
    },
    onUpdate: schedule,
    onSelectionUpdate: () => {
      refresh()
      preview?.selection()
    },
  })
  // Finish extension initialization before installing dirty tracking or publishing readiness.
  if (!editor.isInitialized)
    await new Promise<void>((resolve) => editor.once('create', () => resolve()))
  const bands = new BandHost(editor)
  const openContext = () => {
    if (editor.view.composing) return false
    if (bands.isOpen) {
      bands.focus()
      return true
    }
    // The same boundary that highlights the badge determines this command's target.
    const inline = activeInlineObject(editor.state)
    if (inline) {
      openProperties(inline.pos, inline.node.type.name)
      return true
    }
    if (openCorrection(editor)) return true
    const target = propertyTarget()
    if (!target) return false
    openProperties(target.pos, target.kind)
    return true
  }
  properties = propertyPanel(editor, bands, contentCatalogue, () =>
    element<HTMLInputElement>('image-file').click(),
  )
  tables = tablePanel(
    editor,
    bands,
    provider,
    host.transport,
    contentCatalogue.entries.find((c) => c.id === 'insert.table')!,
  )
  preview = previewController(editor, provider, host.transport, () => version)
  crossReferences = crossReferencePanel(editor, bands)
  citations = citationPanel(editor, provider, host.transport, bands)
  let referenceKey = '',
    referenceTicket = 0
  const refreshReferences = () => {
    if (stopping) return
    const files = referenceRecords(provider),
      key = JSON.stringify(files)
    if (key === referenceKey) return
    referenceKey = key
    const ticket = ++referenceTicket
    void host.transport
      .invoke<ReferenceEntry[]>('bibliography_index', { files })
      .then((entries) => {
        if (ticket === referenceTicket) setCitationEntries(editor, entries)
      })
      .catch((error) => {
        if (ticket === referenceTicket) {
          setCitationEntries(editor, [])
          message(errorText(error), true)
        }
      })
  }
  provider.doc.getMap('settings').observeDeep(() => queueMicrotask(refreshReferences))
  refreshReferences()
  commands = createRegistry(
    editor,
    contentCatalogue.entries,
    openProperties,
    () => element<HTMLInputElement>('image-file').click(),
    () => documentType(provider).capabilities,
    () => citations?.open(),
    () => tables?.open(),
    () => crossReferences?.open(),
  )
  tables.register(commands)
  commands.set('context.open', {
    id: 'context.open',
    label: tr('Paramètres'),
    shortcut: 'Mod+Enter',
    enabled: () =>
      !editor.view.composing &&
      (bands.isOpen || !!correctionAtCursor(editor) || !!propertyTarget()),
    run: openContext,
  })
  commands.set('block.remove', {
    id: 'block.remove',
    label: tr('Supprimer le bloc'),
    shortcut: 'Mod+Backspace',
    enabled: () => !!inlineObjectTarget(editor, 'before') || !!widgetTarget(editor),
    run: () => {
      if (editor.view.composing) return
      const inline = inlineObjectTarget(editor, 'before')
      if (inline) openProperties(inline.pos, inline.node.type.name, true)
      else removeWidget(editor)
    },
  })
  registerRecoveryOpener(provider, editor, (pos, draft) => {
    bands.close(false)
    const details = (draft.data as any)?.data
    if (draft.form === 'proofing-band') {
      const context = draft.context as any
      editor.commands.setTextSelection({
        from: pos,
        to: pos + context.range.to - context.range.from,
      })
      return openCorrection(editor)
    }
    if (editor.state.doc.nodeAt(pos)?.type.name === 'tableFigure')
      return tables?.resume(pos, details) || false
    if (
      details?.new &&
      (draft.form === 'citation-properties' ||
        draft.form === 'cross-reference-properties' ||
        draft.form === 'table-properties')
    ) {
      editor.commands.setTextSelection(pos)
      if (draft.form === 'citation-properties') citations?.open()
      else if (draft.form === 'cross-reference-properties') crossReferences?.open()
      else tables?.open()
      return bands.isOpen
    }
    const kind = details?.kind || editor.state.doc.nodeAt(pos)?.type.name
    if (!kind) return false
    openProperties(pos, kind, !!details?.remove)
    return bands.isOpen
  })
  toolbar(element('toolbar'), commands, contentCatalogue.entries)
  await mountShell({
    editor,
    provider,
    transport: host.transport,
    catalogue: contentCatalogue,
    bands,
    commands,
    save,
    schedule,
    refresh,
    message,
    isTauri,
    opened: bootstrap.opened,
    hasRecovery: bootstrap.hasRecovery,
    filePath: bootstrap.filePath,
    initialPath: bootstrap.initialPath,
    diagnostic:
      recipeBuild &&
      !new URLSearchParams(location.search).has('welcome') &&
      !!(
        browserHost ||
        window.__SPIKE_TRANSPORT__ ||
        (isTauri && diagnosticMode !== 'draft-recover' && diagnosticMode)
      ),
  })
  if (provider.nativeEditing) await provider.clearHistory()
  else {
    yUndoPluginKey.getState(editor.state)?.undoManager.addToScope(provider.doc.getMap('settings'))
    yUndoPluginKey.getState(editor.state)?.undoManager.trackedOrigins.add(ACCEPTED_PROPOSAL)
    historyFor(editor)?.clear()
  }
  if (
    bootstrap.restoredDocument &&
    (stableJSON(
      documentContent(
        normalizeWhitespace(
          EditorState.create({ doc: schema.nodeFromJSON(bootstrap.restoredDocument) }).tr,
        ).doc.toJSON(),
      ),
    ) !== stableJSON(documentContent(normalizeWhitespace(editor.state.tr).doc.toJSON())) ||
      stableJSON(bootstrap.settings) !== stableJSON(settingsFor(provider)))
  ) {
    restoringError = true
    throw new Error(
      'L’état Yjs ne correspond pas à l’instantané sauvegardé. La session existante est conservée.',
    )
  }
  provider.onStatus = refresh
  provider.onChange = () => {
    refresh()
    schedule()
  }
  provider.onResource = () => {
    if (editor) editor.view.dispatch(editor.state.tr)
  }
  const lockHistory = provider.onHistoryLock
  provider.onHistoryLock = (locked) => {
    if (!locked) document.querySelector<HTMLElement>('.writing')!.inert = false
    lockHistory(locked)
    if (locked && !document.activeElement?.closest('#metadata-content'))
      document.querySelector<HTMLElement>('.writing')!.inert = true
  }
  const recovery = draftRecovery(provider, snapshot, isTauri)
  provider.onError = (error) => {
    recovery.refresh()
    message(
      provider.rejected
        ? tr('La synchronisation est suspendue ; votre brouillon est conservé.')
        : errorText(error),
      true,
    )
    refresh()
  }
  provider.onSynchronization = refresh
  provider.presence.observe(refresh)
  if (recipeBuild) window.__spike = { editor, provider, save, ready: true }
  refresh()
  schedule()
  message(
    tr(
      provider.synchronization?.enabled
        ? 'Synchronisation active'
        : 'Mode individuel · validation et historique actifs',
    ),
  )
  if (recipeBuild && isTauri) {
    const mode = diagnosticMode
    if (mode?.startsWith('paragraph-input')) {
      void (await import('./paragraph-input-diagnostic')).paragraphInputDiagnostic(
        mode,
        host.transport,
        provider,
        editor,
      )
    } else if (mode?.startsWith('synchronization') && synchronizationMeter) {
      void (await import('./synchronization-performance')).synchronizationPerformance(
        host.transport,
        provider,
        editor,
        synchronizationMeter,
        rendering.pause,
        snapshot,
        mode === 'synchronization-active',
      )
    } else if (mode?.startsWith('draft-')) {
      void (await import('./draft-diagnostic')).draftDiagnostic(
        mode,
        host.transport,
        provider,
        editor,
      )
    } else if (mode?.startsWith('publication')) {
      void (await import('./publication-diagnostic')).publicationDiagnostic(
        mode,
        host.transport,
        provider,
        editor,
      )
    } else if (mode?.startsWith('closure')) {
      void (await import('./closure-diagnostic')).closureDiagnostic(
        mode,
        host.transport,
        provider,
        editor,
      )
    } else if (mode === 'performance') {
      void (await import('./preview-performance')).nativePreviewPerformance(
        host.transport,
        provider,
        editor,
        {
          pause: rendering.pause,
          resume: () => {
            rendering.resume()
            rendering.request()
          },
          snapshot,
          show: (result) => preview!.show(result, version),
          observe: (callback) => {
            onPreview = callback
          },
        },
      )
    } else if (mode)
      void (await import('./native-diagnostic')).nativeDiagnostic(
        mode,
        host.transport,
        provider,
        editor,
        save,
      )
  }
}
void start().catch((error) => message(errorText(error), true))
window.addEventListener('beforeunload', () => {
  stopping = true
  interfaceRefresh.stop()
  statistics.stop()
  preview?.destroy()
  provider.destroy()
})

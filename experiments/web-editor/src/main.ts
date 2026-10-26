import { initializeUI, tr } from './i18n'
import { Editor, getSchema, type JSONContent } from '@tiptap/core'
import { prosemirrorJSONToYDoc, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { baseExtensions, collaborativeExtensions } from './extensions'
import { Provider, stableJSON, ACCEPTED_PROPOSAL, type Resource, type Transport } from './provider'
import { createRegistry, toolbar, refreshCommands, type Registry } from './commands'
import { widgetKey, widgetTarget, removeWidget } from './widgets'
import { openCorrection, correctionAtCursor } from './proofing'
import { widgetPresentations } from './widget-frame'
import { hasSettings, settingsForNode } from './capability-settings'
import { mountShell, type Catalogue } from './shell'
import { documentType, initializeSettings, settingsFor } from './state'
import { previewController } from './preview'
import { requestPreview } from './preview-frame'
import { renderQueue } from './render-queue'
import { installMenus } from './menus'
import { BandHost } from './band-layout'
import { propertyPanel } from './properties'
import { insertBlock } from './editing'
import './style.css'

declare global {
  interface Window {
    __SPIKE_TRANSPORT__?: { id: string; transport: Transport }
    __spike?: { editor: Editor; provider: Provider; save: () => Promise<void>; ready: boolean }
  }
}

const app = document.querySelector<HTMLDivElement>('#app')!
installMenus()
app.innerHTML = `
  <header><div class="identity"><strong>typstOgraphe</strong><span class="badge">Diagnostic web</span></div>
    <div class="session-tools"><button id="peer">Deuxième fenêtre</button><button id="connection" aria-pressed="true">Connecté</button><button id="save">Enregistrer</button><button id="export">Exporter</button></div></header>
  <main><section class="writing" aria-label="Éditeur du document">
    <nav id="toolbar" aria-label="Mise en forme"></nav>
    <div class="document-info"><span>Courrier personnel</span><span>Français · A4 · Modèle local</span></div>
    <div id="editor-scroll"><div id="editor"></div></div>
  </section><div class="splitter" role="separator" tabindex="0" aria-label="Largeur des colonnes" aria-orientation="vertical" aria-valuemin="10" aria-valuemax="90" aria-valuenow="50"></div>
  <section class="preview" aria-label="Aperçu Typst"><div class="preview-title"><strong>Aperçu Typst</strong><span id="render-status" role="status">Préparation…</span></div><div id="pages"><p class="empty">Le rendu du document apparaîtra ici.</p></div></section></main>
  <footer><span id="statistics">Chargement…</span><span id="presence"></span><span id="message" role="status"></span></footer>
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
const host = window.__SPIKE_TRANSPORT__ ??
  browserHost ?? {
    id: isTauri ? getCurrentWebviewWindow().label : 'browser',
    transport: {
      invoke: <T>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args),
      listen: async <T>(name: string, handler: (payload: T) => void) =>
        listen<T>(name, (event) => handler(event.payload)),
    } satisfies Transport,
  }
const provider = new Provider(host.id, host.transport)
let editor: Editor
let properties: ReturnType<typeof propertyPanel> | undefined
const rendering = renderQueue(render)
let scheduledAt = 0
let onPreview: ((revision: number) => void) | undefined
let restoringError = false
let statisticsTicket = 0
let version = 0
let preview: ReturnType<typeof previewController> | undefined

function propertyTarget(): { pos: number; kind: string } | undefined {
  const { from } = editor.state.selection
  const at = editor.state.doc.nodeAt(from)
  const widget = widgetTarget(editor)
  const target =
    at?.type.name === 'footnote'
      ? { pos: from, kind: 'footnote' }
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
  properties?.open(pos, kind, remove)
}

let commands: Registry = new Map()
let contentCatalogue: Catalogue
const tableTools = document.createElement('div')
tableTools.className = 'table-tools'
tableTools.dataset.active = 'false'
for (const [label, command] of [
  [tr('Ajouter une ligne'), 'addRowAfter'],
  [tr('Retirer la ligne'), 'deleteRow'],
  [tr('Ajouter une colonne'), 'addColumnAfter'],
  [tr('Retirer la colonne'), 'deleteColumn'],
  [tr('Supprimer le tableau'), 'deleteTable'],
] as const) {
  const button = document.createElement('button')
  button.type = 'button'
  button.textContent = label
  button.addEventListener('click', () => {
    if (command === 'deleteTable') {
      const target = widgetTarget(editor)
      if (target?.node.type.name === 'table') openProperties(target.pos, 'table', true)
    } else editor.chain().focus()[command]().run()
  })
  tableTools.append(button)
}
element('toolbar').after(tableTools)

function refresh() {
  if (!editor) return
  refreshCommands(commands)
  tableTools.dataset.active = String(editor.isActive('table'))
  const leaf = (node: import('@tiptap/pm/model').Node) =>
    node.type.name === 'footnote'
      ? `\n${node.attrs.text || ''}\n`
      : node.type.name === 'softBreak'
        ? '\u2028'
        : '\n'
  const text = editor.state.doc.textBetween(0, editor.state.doc.content.size, '\n', leaf)
  const { from, to, empty } = editor.state.selection
  const selection = empty ? null : editor.state.doc.textBetween(from, to, '\n', leaf)
  const ticket = ++statisticsTicket
  void host.transport
    .invoke<{
      label: string
      hint: string
      document: { words: number; signs: number; signs_without_spaces: number }
      selection: { words: number; signs: number; signs_without_spaces: number } | null
    }>('count_text', { text, selection })
    .then((counts) => {
      if (ticket !== statisticsTicket) return
      element('statistics').textContent = counts.label
      element('statistics').title = counts.hint
    })
    .catch(() => {})
  const people = [...provider.awareness.getStates().values()]
    .map((s) => s.user?.name)
    .filter(Boolean)
  element('presence').textContent = people.join(' · ')
  const connection = element<HTMLButtonElement>('connection')
  connection.textContent = provider.online ? tr('Connecté') : tr('Hors connexion')
  connection.setAttribute('aria-pressed', String(provider.online))
}
function schedule() {
  version++
  preview?.invalidate()
  scheduledAt = performance.now()
  if (!provider.online) {
    element('render-status').textContent = tr('Modifications locales · hors connexion')
    return
  }
  element('render-status').textContent = tr('Mise à jour…')
  rendering.request()
}
async function render() {
  if (!editor || !provider.online || restoringError) return
  const localVersion = version
  const inputTime = scheduledAt
  try {
    await provider.flush()
    if (localVersion !== version) return
    const { result } = await requestPreview(host.transport, snapshot())
    if (localVersion !== version) return
    if (!(await preview?.show(result, localVersion)) || localVersion !== version) return
    const pages = result.pages
    element('render-status').textContent =
      `${pages.length} page${pages.length > 1 ? 's' : ''} · ${Math.round(result.elapsedMs)} ms`
    element('render-status').classList.remove('error')
    element('render-status').title =
      `${Math.round(performance.now() - inputTime)} ms · édition → publication`
    onPreview?.(result.revision)
  } catch (error) {
    if (String(error) === 'stale') {
      rendering.request()
      return
    }
    element('render-status').textContent = tr('Contenu à corriger · dernier rendu conservé')
    element('render-status').classList.add('error')
    message(String(error), true)
  }
}
function snapshot() {
  return {
    revision: provider.revision,
    document: editor.getJSON(),
    settings: settingsFor(provider),
  }
}
async function save() {
  if (!provider.online || restoringError)
    throw new Error(tr('Reconnectez cette fenêtre avant d’enregistrer.'))
  await provider.flush()
  const path = await host.transport.invoke<string>('save', {
    snapshot: snapshot(),
    update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
  })
  message(`${tr('Session enregistrée')} · ${path}`)
}
function action(task: () => Promise<unknown>) {
  void task().catch((error) => message(String(error), true))
}
element('save').addEventListener('click', () => action(save))
element('peer').addEventListener('click', () => action(() => host.transport.invoke('open_peer')))
element('export').addEventListener('click', () =>
  action(async () => {
    if (!provider.online) throw new Error(tr('Reconnectez cette fenêtre avant d’exporter.'))
    await provider.flush()
    const path = await host.transport.invoke<string>('export_document', {
      snapshot: snapshot(),
    })
    message(`${tr('PDF et sources Typst exportés')} · ${path}`)
  }),
)
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
    const extension = file.name.split('.').pop()?.toLowerCase() || ''
    const resource = await host.transport.invoke<Resource>('import_image', {
      extension,
      data: Array.from(new Uint8Array(await file.arrayBuffer())),
    })
    provider.addResource(resource)
    if (properties?.replaceImage(resource.path)) {
      element<HTMLInputElement>('image-file').value = ''
      return
    }
    insertBlock(editor, { type: 'figure', attrs: { src: resource.path, alt: '' }, content: [] })
    element<HTMLInputElement>('image-file').value = ''
  }),
)
const rustButton = document.createElement('button')
rustButton.id = 'rust-experiment'
rustButton.type = 'button'
rustButton.textContent = tr('Essai Rust')
rustButton.title = 'Insérer une proposition préparée côté Rust dans l’état partagé'
rustButton.addEventListener('click', () =>
  action(async () => {
    const content = await host.transport.invoke<JSONContent>('prepare_rust_edit')
    editor.chain().focus().insertContentAt(editor.state.doc.content.size, content).run()
  }),
)
element('statistics').after(rustButton)
document.addEventListener('keydown', (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    action(save)
  }
})

const splitter = document.querySelector<HTMLElement>('.splitter')!
function resize(fraction: number) {
  const value = Math.max(10, Math.min(90, fraction))
  document.documentElement.style.setProperty('--editor-width', `${value}%`)
  splitter.setAttribute('aria-valuenow', String(Math.round(value)))
}
splitter.addEventListener('pointerdown', (event) => {
  splitter.setPointerCapture(event.pointerId)
  const move = (e: PointerEvent) => resize((e.clientX / window.innerWidth) * 100)
  const end = () => {
    splitter.removeEventListener('pointermove', move)
    splitter.removeEventListener('pointerup', end)
  }
  splitter.addEventListener('pointermove', move)
  splitter.addEventListener('pointerup', end)
})
splitter.addEventListener('keydown', (event) => {
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    event.preventDefault()
    resize(Number(splitter.getAttribute('aria-valuenow')) + (event.key === 'ArrowLeft' ? -2 : 2))
  }
})
splitter.addEventListener('dblclick', () => resize(50))

async function start() {
  if (!isTauri && !window.__SPIKE_TRANSPORT__ && !browserHost)
    throw new Error(
      'Lancez ce diagnostic avec « npm run tauri dev ». Le navigateur seul ne fournit pas le cœur Rust.',
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
  document.querySelector('.preview-title strong')!.textContent = tr('Aperçu Typst')
  const presentations = widgetPresentations(contentCatalogue.entries)
  const schema = getSchema(
    baseExtensions((path) => provider.imageURL(path), undefined, presentations),
  )
  const bootstrap = await provider.start((fixture) => {
    const seed = prosemirrorJSONToYDoc(schema, fixture, 'default')
    const update = Y.encodeStateAsUpdate(seed)
    seed.destroy()
    return update
  })
  editor = new Editor({
    element: element('editor'),
    extensions: await collaborativeExtensions(provider, openProperties, presentations),
    editorProps: {
      attributes: {
        'aria-label': tr('Corps du document'),
        role: 'textbox',
        'aria-multiline': 'true',
        // Spellbook owns document underlines. Keep native services available
        // on explicit request, without a second checker or silent replacements.
        spellcheck: 'false',
        autocorrect: 'off',
      },
      handleKeyDown: (_view, event) => {
        if (event.isComposing || editor.view.composing) return false
        if (widgetKey(editor, event)) return true
        if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
          event.preventDefault()
          openContext()
          return true
        }
        return false
      },
    },
    onTransaction: ({ transaction }) => {
      refresh()
    },
    onUpdate: schedule,
    onSelectionUpdate: () => {
      refresh()
      preview?.selection()
    },
  })
  const bands = new BandHost(editor)
  const openContext = () => {
    if (editor.view.composing) return false
    if (bands.isOpen) {
      bands.focus()
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
  preview = previewController(editor, provider, host.transport, () => version)
  initializeSettings(provider, contentCatalogue, bootstrap.settings)
  commands = createRegistry(
    editor,
    contentCatalogue.entries,
    openProperties,
    () => element<HTMLInputElement>('image-file').click(),
    () => documentType(provider).capabilities,
  )
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
    enabled: () => !!widgetTarget(editor),
    run: () => {
      if (!editor.view.composing) removeWidget(editor)
    },
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
    message,
    isTauri,
    opened: bootstrap.opened,
    hasRecovery: bootstrap.hasRecovery,
    filePath: bootstrap.filePath,
    diagnostic:
      !new URLSearchParams(location.search).has('welcome') &&
      !!(
        browserHost ||
        window.__SPIKE_TRANSPORT__ ||
        (isTauri && (await host.transport.invoke('diagnostic_mode')))
      ),
  })
  yUndoPluginKey.getState(editor.state)?.undoManager.addToScope(provider.doc.getMap('settings'))
  yUndoPluginKey.getState(editor.state)?.undoManager.trackedOrigins.add(ACCEPTED_PROPOSAL)
  yUndoPluginKey.getState(editor.state)?.undoManager.clear()
  if (
    bootstrap.restoredDocument &&
    (stableJSON(schema.nodeFromJSON(bootstrap.restoredDocument).toJSON()) !==
      stableJSON(editor.getJSON()) ||
      stableJSON(bootstrap.settings) !== stableJSON(settingsFor(provider)))
  ) {
    restoringError = true
    throw new Error(
      'L’état Yjs ne correspond pas à l’instantané sauvegardé. La session existante est conservée.',
    )
  }
  provider.onChange = () => {
    refresh()
    schedule()
  }
  provider.onResource = () => {
    if (editor) editor.view.dispatch(editor.state.tr)
  }
  provider.onError = (error) => {
    message(String(error), true)
    refresh()
  }
  provider.awareness.on('change', refresh)
  window.__spike = { editor, provider, save, ready: true }
  refresh()
  schedule()
  message('Session locale · modifications partagées entre les fenêtres ouvertes')
  if (isTauri) {
    const mode = await host.transport.invoke<string | null>('diagnostic_mode')
    if (mode === 'performance') {
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
void start().catch((error) => message(String(error), true))
window.addEventListener('beforeunload', () => {
  preview?.destroy()
  provider.destroy()
})

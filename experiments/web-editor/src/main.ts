import { Editor, getSchema, type JSONContent } from '@tiptap/core'
import { prosemirrorJSONToYDoc, yUndoPluginKey } from '@tiptap/y-tiptap'
import * as Y from 'yjs'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { baseExtensions, collaborativeExtensions } from './extensions'
import { Provider, stableJSON, type Resource, type Transport } from './provider'
import './style.css'

export interface Preview {
  revision: number
  source: string
  pages: string[]
  elapsedMs: number
}
declare global {
  interface Window {
    __SPIKE_TRANSPORT__?: { id: string; transport: Transport }
    __spike?: { editor: Editor; provider: Provider; save: () => Promise<void>; ready: boolean }
  }
}

const app = document.querySelector<HTMLDivElement>('#app')!
app.innerHTML = `
  <header><div class="identity"><strong>typstOgraphe</strong><span class="badge">Diagnostic web</span></div>
    <div class="session-tools"><button id="peer">Deuxième fenêtre</button><button id="connection" aria-pressed="true">Connecté</button><button id="save">Enregistrer</button><button id="export">Exporter</button></div></header>
  <main><section class="writing" aria-label="Éditeur du document">
    <nav id="toolbar" aria-label="Mise en forme"></nav>
    <div class="document-info"><span>Courrier personnel</span><span>Français · A4 · Modèle local</span></div>
    <form id="properties" class="properties" hidden><div class="panel-title"><strong id="property-title"></strong><button type="button" id="close-properties" aria-label="Fermer les paramètres">×</button></div><label id="property-label" for="property-value"></label><textarea id="property-value" rows="3"></textarea><div class="panel-actions"><button type="button" id="remove-block" class="danger">Supprimer</button><span></span><button type="button" id="cancel-properties">Annuler</button><button type="submit" class="primary">Appliquer</button></div><p id="property-error" role="alert"></p></form>
    <div id="editor-scroll"><div id="editor"></div></div>
  </section><div class="splitter" role="separator" tabindex="0" aria-label="Largeur des colonnes" aria-orientation="vertical" aria-valuemin="30" aria-valuemax="70" aria-valuenow="50"></div>
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
let property: { pos: number; kind: string; original: string } | undefined
let timer: ReturnType<typeof setTimeout>
let rendering = false
let dirty = false
let restoringError = false
let version = 0

function propertyTarget(): { pos: number; kind: string } | undefined {
  const { $from, from } = editor.state.selection
  const at = editor.state.doc.nodeAt(from)
  if (at?.type.name === 'footnote') return { pos: from, kind: 'footnote' }
  if (editor.isActive('link')) return { pos: from, kind: 'link' }
  for (let depth = $from.depth; depth > 0; depth--) {
    if ($from.node(depth).type.name === 'figure')
      return { pos: $from.before(depth), kind: 'figure' }
  }
  return undefined
}
function openProperties(pos: number, kind: string) {
  const node = editor.state.doc.nodeAt(pos)
  if (!node || (kind !== 'link' && node.type.name !== kind)) return
  const original =
    kind === 'link'
      ? editor.getAttributes('link').href || ''
      : kind === 'figure'
        ? node.attrs.alt
        : node.attrs.text
  property = { pos, kind, original }
  element('property-title').textContent =
    kind === 'link' ? 'Lien' : kind === 'figure' ? 'Paramètres de l’image' : 'Note de bas de page'
  element('property-label').textContent =
    kind === 'link'
      ? 'Adresse du lien'
      : kind === 'figure'
        ? 'Texte alternatif'
        : 'Texte de la note'
  element<HTMLTextAreaElement>('property-value').value = original
  element('property-error').textContent = ''
  element('properties').hidden = false
  element<HTMLTextAreaElement>('property-value').focus()
}
function closeProperties(focus = true) {
  const pos = property?.pos
  property = undefined
  element('properties').hidden = true
  if (focus && pos !== undefined)
    editor.chain().focus().setTextSelection(Math.min(pos, editor.state.doc.content.size)).run()
}
element<HTMLFormElement>('properties').addEventListener('submit', (event) => {
  event.preventDefault()
  if (!property) return
  const { pos, kind, original } = property
  if (kind === 'link') {
    const url = element<HTMLTextAreaElement>('property-value').value.trim()
    if (!['https://', 'http://', 'mailto:'].some((prefix) => url.startsWith(prefix))) {
      element('property-error').textContent = 'Utilisez une adresse http, https ou mailto.'
      return
    }
    editor.chain().focus().extendMarkRange('link').setLink({ href: url }).run()
    closeProperties()
    return
  }
  const node = editor.state.doc.nodeAt(pos)
  const key = kind === 'figure' ? 'alt' : 'text'
  if (!node || node.type.name !== kind || node.attrs[key] !== original) {
    element('property-error').textContent =
      'Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.'
    return
  }
  const value = element<HTMLTextAreaElement>('property-value').value
  editor.view.dispatch(
    editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, [key]: value }),
  )
  closeProperties()
})
element('properties').addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    event.preventDefault()
    closeProperties()
  }
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    element<HTMLFormElement>('properties').requestSubmit()
  }
})
for (const id of ['close-properties', 'cancel-properties'])
  element(id).addEventListener('click', () => closeProperties())
element('remove-block').addEventListener('click', () => {
  if (!property) return
  if (property.kind === 'link') {
    editor.chain().focus().extendMarkRange('link').unsetLink().run()
    closeProperties()
    return
  }
  const node = editor.state.doc.nodeAt(property.pos)
  if (node?.type.name === property.kind)
    editor.view.dispatch(editor.state.tr.delete(property.pos, property.pos + node.nodeSize))
  closeProperties()
})

const buttons: { label: string; text: string; action: () => void; active?: () => boolean }[] = [
  {
    label: 'Paragraphe',
    text: '¶',
    action: () => {
      editor.chain().focus().setParagraph().run()
    },
  },
  ...([1, 2, 3] as const).map((level) => ({
    label: `Titre ${level}`,
    text: `H${level}`,
    action: () => {
      editor.chain().focus().toggleHeading({ level }).run()
    },
    active: () => editor.isActive('heading', { level }),
  })),
  {
    label: 'Gras',
    text: 'B',
    action: () => {
      editor.chain().focus().toggleBold().run()
    },
    active: () => editor.isActive('bold'),
  },
  {
    label: 'Italique',
    text: 'I',
    action: () => {
      editor.chain().focus().toggleItalic().run()
    },
    active: () => editor.isActive('italic'),
  },
  {
    label: 'Lien',
    text: '↗',
    action: () => openProperties(editor.state.selection.from, 'link'),
    active: () => editor.isActive('link'),
  },
  {
    label: 'Liste à puces',
    text: '• ≡',
    action: () => {
      editor.chain().focus().toggleBulletList().run()
    },
    active: () => editor.isActive('bulletList'),
  },
  {
    label: 'Liste numérotée',
    text: '1 ≡',
    action: () => {
      editor.chain().focus().toggleOrderedList().run()
    },
    active: () => editor.isActive('orderedList'),
  },
  {
    label: 'Insérer un tableau',
    text: '▦',
    action: () => {
      editor.chain().focus().insertTable({ rows: 3, cols: 2, withHeaderRow: true }).run()
    },
  },
  {
    label: 'Insérer une note',
    text: '¹',
    action: () => {
      editor
        .chain()
        .focus()
        .insertContent({ type: 'footnote', attrs: { text: '' } })
        .run()
      const pos = editor.state.selection.from - 1
      openProperties(pos, 'footnote')
    },
  },
  {
    label: 'Insérer une image',
    text: '▧',
    action: () => element<HTMLInputElement>('image-file').click(),
  },
  {
    label: 'Annuler',
    text: '↶',
    action: () => {
      editor.chain().focus().undo().run()
    },
  },
  {
    label: 'Rétablir',
    text: '↷',
    action: () => {
      editor.chain().focus().redo().run()
    },
  },
]
for (const item of buttons) {
  const button = document.createElement('button')
  button.type = 'button'
  button.title = item.label
  button.setAttribute('aria-label', item.label)
  button.textContent = item.text
  button.addEventListener('click', () => item.action())
  element('toolbar').append(button)
}
const tableTools = document.createElement('div')
tableTools.className = 'table-tools'
tableTools.dataset.active = 'false'
for (const [label, command] of [
  ['Ajouter une ligne', 'addRowAfter'],
  ['Retirer la ligne', 'deleteRow'],
  ['Ajouter une colonne', 'addColumnAfter'],
  ['Retirer la colonne', 'deleteColumn'],
  ['Supprimer le tableau', 'deleteTable'],
] as const) {
  const button = document.createElement('button')
  button.type = 'button'
  button.textContent = label
  button.addEventListener('click', () => {
    editor.chain().focus()[command]().run()
  })
  tableTools.append(button)
}
element('toolbar').after(tableTools)

function refresh() {
  if (!editor) return
  for (const [index, item] of buttons.entries()) {
    if (item.active)
      element('toolbar').children[index].setAttribute('aria-pressed', String(item.active()))
  }
  tableTools.dataset.active = String(editor.isActive('table'))
  const text = editor.getText()
  const words = [...new Intl.Segmenter('fr', { granularity: 'word' }).segment(text)].filter(
    (s) => s.isWordLike,
  ).length
  const signs = [...new Intl.Segmenter('fr', { granularity: 'grapheme' }).segment(text)].length
  element('statistics').textContent = `${words} mots · ${signs} signes`
  const people = [...provider.awareness.getStates().values()]
    .map((s) => s.user?.name)
    .filter(Boolean)
  element('presence').textContent = people.join(' · ')
  const connection = element<HTMLButtonElement>('connection')
  connection.textContent = provider.online ? 'Connecté' : 'Hors connexion'
  connection.setAttribute('aria-pressed', String(provider.online))
}
function schedule() {
  version++
  dirty = true
  if (!provider.online) {
    element('render-status').textContent = 'Modifications locales · hors connexion'
    return
  }
  element('render-status').textContent = 'Mise à jour…'
  clearTimeout(timer)
  timer = setTimeout(() => void render(), 180)
}
async function render() {
  if (!editor || rendering || !provider.online || restoringError) return
  rendering = true
  dirty = false
  const localVersion = version
  try {
    await provider.flush()
    const result = await host.transport.invoke<Preview>('preview', {
      snapshot: { revision: provider.revision, document: editor.getJSON() },
    })
    if (localVersion !== version) return
    const pages = result.pages.map((src, index) => {
      const image = document.createElement('img')
      image.src = src
      image.alt = `Page ${index + 1} du rendu Typst`
      return image
    })
    element('pages').replaceChildren(...pages)
    element('render-status').textContent =
      `${pages.length} page${pages.length > 1 ? 's' : ''} · ${Math.round(result.elapsedMs)} ms`
    element('render-status').classList.remove('error')
  } catch (error) {
    if (String(error) === 'stale') {
      dirty = true
      return
    }
    element('render-status').textContent = 'Contenu à corriger · dernier rendu conservé'
    element('render-status').classList.add('error')
    message(String(error), true)
  } finally {
    rendering = false
    if (dirty) timer = setTimeout(() => void render(), 180)
  }
}
async function save() {
  if (!provider.online || restoringError)
    throw new Error('Reconnectez cette fenêtre avant d’enregistrer.')
  await provider.flush()
  const path = await host.transport.invoke<string>('save', {
    snapshot: { revision: provider.revision, document: editor.getJSON() },
    update: Array.from(Y.encodeStateAsUpdate(provider.doc)),
  })
  message(`Session enregistrée · ${path}`)
}
function action(task: () => Promise<unknown>) {
  void task().catch((error) => message(String(error), true))
}
element('save').addEventListener('click', () => action(save))
element('peer').addEventListener('click', () => action(() => host.transport.invoke('open_peer')))
element('export').addEventListener('click', () =>
  action(async () => {
    if (!provider.online) throw new Error('Reconnectez cette fenêtre avant d’exporter.')
    await provider.flush()
    const path = await host.transport.invoke<string>('export_document', {
      snapshot: { revision: provider.revision, document: editor.getJSON() },
    })
    message(`PDF et sources Typst exportés · ${path}`)
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
    editor
      .chain()
      .focus()
      .insertContent({ type: 'figure', attrs: { src: resource.path, alt: '' }, content: [] })
      .run()
    element<HTMLInputElement>('image-file').value = ''
  }),
)
const rustButton = document.createElement('button')
rustButton.type = 'button'
rustButton.textContent = 'Essai Rust'
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
  const value = Math.max(30, Math.min(70, fraction))
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
  const schema = getSchema(baseExtensions((path) => provider.imageURL(path)))
  const bootstrap = await provider.start((fixture) => {
    const seed = prosemirrorJSONToYDoc(schema, fixture, 'default')
    const update = Y.encodeStateAsUpdate(seed)
    seed.destroy()
    return update
  })
  editor = new Editor({
    element: element('editor'),
    extensions: await collaborativeExtensions(provider, openProperties),
    editorProps: {
      attributes: {
        'aria-label': 'Corps du document',
        role: 'textbox',
        'aria-multiline': 'true',
        spellcheck: 'true',
      },
      handleKeyDown: (_view, event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
          const target = propertyTarget()
          if (target) {
            event.preventDefault()
            openProperties(target.pos, target.kind)
            return true
          }
        }
        return false
      },
    },
    onTransaction: ({ transaction }) => {
      if (property) {
        property.pos = transaction.mapping.map(property.pos)
        if (
          property.kind !== 'link' &&
          editor.state.doc.nodeAt(property.pos)?.type.name !== property.kind
        )
          closeProperties(false)
      }
      refresh()
    },
    onUpdate: schedule,
    onSelectionUpdate: refresh,
  })
  yUndoPluginKey.getState(editor.state)?.undoManager.clear()
  if (
    bootstrap.restoredDocument &&
    stableJSON(schema.nodeFromJSON(bootstrap.restoredDocument).toJSON()) !==
      stableJSON(editor.getJSON())
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
    if (mode)
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
window.addEventListener('beforeunload', () => provider.destroy())

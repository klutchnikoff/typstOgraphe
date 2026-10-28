import { metadataCommands } from './metadata-commands'
import type { StructuredCommand } from './structured-commands'
import { bindMetadataInput } from './metadata-input'
import { documentContent } from './shared-inline-marks'
import type { Editor } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import type { DocumentType, ModelPolicy } from './shell'
import { settingsFor } from './state'
import { referenceFiles, type ReferenceFile } from './citation-index'
import { contextPanel } from './context-panel'
import { actionRow } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import { draftGuard } from './snapshot-operation'

export function bibliographyPanel(
  parent: HTMLElement,
  provider: Provider,
  editor: Editor,
  transport: Transport,
  kind: DocumentType,
  policy: () => ModelPolicy | undefined,
) {
  const commands = metadataCommands(editor, provider)
  const schema = kind.structured!,
    descriptor = schema.bibliography!
  const definition = schema.collections.find(
    (collection) => collection.parameter === descriptor.files,
  )!
  const root = document.createElement('div')
  root.className = 'bibliography-settings'
  parent.append(root)
  const style = document.createElement('label')
  style.className = 'bibliography-style'
  const select = document.createElement('select')
  select.setAttribute('aria-label', tr('Style bibliographique'))
  style.append(document.createTextNode(tr('Style bibliographique')), select)
  select.onchange = () => {
    commands.set(
      { kind: 'metadata', field: descriptor.style_field },
      select.value,
      () => settingsFor(provider).metadata.fields[descriptor.style],
    )
  }
  const rows = document.createElement('div'),
    actions = actionRow(),
    status = document.createElement('p')
  rows.className = 'bibliography-files'
  status.role = 'alert'
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.bib,.yaml,.yml'
  input.hidden = true
  input.setAttribute('aria-label', tr('Fichiers bibliographiques'))
  const add = document.createElement('button')
  add.type = 'button'
  add.textContent = tr('Ajouter des fichiers')
  actions.append(add)
  let replacing: ReferenceFile | undefined,
    alive = true,
    busy = false
  const choose = (file?: ReferenceFile) => {
    replacing = file
    input.multiple = !file
    input.value = ''
    input.click()
  }
  add.onclick = () => choose()
  const confirm = contextPanel({
    id: 'remove-bibliography-file',
    closeLabel: tr('Annuler'),
    close: () => {
      confirm.element.hidden = true
      add.focus()
    },
    canConfirm: () => !busy,
  })
  confirm.title.textContent = tr('Retirer le fichier bibliographique')
  let removing: ReferenceFile | undefined
  confirm.action({
    label: tr('Annuler'),
    run: () => {
      confirm.element.hidden = true
      add.focus()
    },
  })
  const publish = async (
    next: ReferenceFile[],
    batch: StructuredCommand[],
    active = () => true,
  ) => {
    if (busy) return false
    busy = true
    refresh()
    status.textContent = ''
    confirm.error.textContent = ''
    const before = JSON.stringify(referenceFiles(provider)),
      current = settingsFor(provider),
      document = documentContent(editor.getJSON())
    current.metadata.fields[descriptor.files] = next
    try {
      const check = await draftGuard(
        provider,
        () =>
          before === JSON.stringify(referenceFiles(provider)) &&
          editor.state.doc.eq(editor.schema.nodeFromJSON(document)),
      )
      if (!alive || !active()) return false
      await transport.invoke('validate_bibliography', { document, metadata: current.metadata })
      if (!alive || !active()) return false
      check()
      await commands.run(
        batch,
        () =>
          alive &&
          active() &&
          before === JSON.stringify(referenceFiles(provider)) &&
          editor.state.doc.eq(editor.schema.nodeFromJSON(document)),
      )
      confirm.element.hidden = true
      refresh()
      return true
    } catch (error) {
      if (alive) {
        status.textContent = String(error)
        confirm.error.textContent = String(error)
      }
      return false
    } finally {
      busy = false
      if (alive) refresh()
    }
  }
  confirm.action({
    label: tr('Retirer'),
    danger: true,
    primary: true,
    run: () => {
      const file = removing
      if (!file) return
      if (
        JSON.stringify(referenceFiles(provider).find((current) => current.id === file.id)) !==
        JSON.stringify(file)
      ) {
        confirm.error.textContent = tr('Le document a changé. Réessayez.')
        return
      }
      void publish(
        referenceFiles(provider).filter((f) => f.id !== file.id),
        [{ id: 'record.remove', params: { collection: definition.parameter, id: file.id } }],
        () => removing === file && !confirm.element.hidden,
      ).then((ok) => {
        if (ok) add.focus()
      })
    },
  })
  input.onchange = async () => {
    const chosen = [...(input.files || [])],
      previous = replacing
    if (!chosen.length) return
    if (chosen.some((file) => file.size > 2 * 1024 * 1024)) {
      status.textContent = tr('Fichier trop volumineux (2 Mio maximum).')
      return
    }
    try {
      const files = await Promise.all(
        chosen.map(async (file) => ({
          id: previous?.id || crypto.randomUUID(),
          name: file.name,
          contents: new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(
            await file.arrayBuffer(),
          ),
        })),
      )
      if (!alive) return
      if (
        previous &&
        JSON.stringify(referenceFiles(provider).find((file) => file.id === previous.id)) !==
          JSON.stringify(previous)
      )
        throw new Error(tr('Le document a changé. Réessayez.'))
      const next = previous
        ? referenceFiles(provider).map((file) => (file.id === previous.id ? files[0] : file))
        : [...referenceFiles(provider), ...files]
      const batch: StructuredCommand[] = previous
        ? ['name', 'contents'].map((field) => ({
            id: 'metadata.set',
            params: {
              target: { kind: 'record', collection: definition.parameter, id: previous.id, field },
              value: files[0][field as 'name' | 'contents'],
            },
          }))
        : files.map(({ id, ...values }) => ({
            id: 'record.add',
            params: { collection: definition.parameter, id, values },
          }))
      await publish(next, batch)
    } catch (error) {
      if (alive) status.textContent = String(error)
    }
  }
  root.append(style, rows, actions, input, confirm.element, status)
  const styleBinding = bindMetadataInput(
    select,
    commands,
    { kind: 'metadata', field: descriptor.style_field },
    () => settingsFor(provider).metadata.fields[descriptor.style],
  )
  function refresh() {
    add.disabled = busy
    select.disabled = busy
    const allowed = policy()?.citation_styles || []
    style.hidden = allowed.length < 2
    if ([...select.options].map((option) => option.value).join() !== allowed.join()) {
      select.replaceChildren()
      const names: Record<string, string> = {
        'chicago-author-date': tr('Chicago auteur–date'),
        apa: 'APA',
        ieee: tr('IEEE (numérique)'),
      }
      for (const value of allowed) select.add(new Option(names[value] || value, value))
    }
    const current = String(settingsFor(provider).metadata.fields[descriptor.style])
    select.value = allowed.includes(current) ? current : allowed[0] || ''
    styleBinding.refresh()
    const files = referenceFiles(provider)
    for (const child of [...rows.children])
      if (!files.some((file) => file.id === (child as HTMLElement).dataset.fileId)) child.remove()
    for (const file of files) {
      let row = rows.querySelector<HTMLElement>(`[data-file-id="${file.id}"]`)
      if (!row) {
        row = document.createElement('div')
        row.className = 'bibliography-file-row'
        row.dataset.fileId = file.id
        row.append(document.createElement('span'), actionRow())
        rows.append(row)
      }
      row.querySelector('span')!.textContent = file.name
      const actions = row.querySelector<HTMLElement>('.panel-actions')!
      for (const [index, [name, glyph, run]] of (
        [
          [tr('Remplacer'), 'redo-2', () => choose(file)],
          [
            tr('Retirer'),
            'trash-2',
            () => {
              removing = file
              confirm.body.textContent = file.name
              confirm.error.textContent = ''
              confirm.element.hidden = false
              confirm.focus()
            },
          ],
        ] as const
      ).entries()) {
        const button =
          (actions.children[index] as HTMLButtonElement | undefined) ||
          document.createElement('button')
        button.type = 'button'
        button.innerHTML = icon(glyph)
        button.title = `${name} · ${file.name}`
        button.setAttribute('aria-label', button.title)
        button.onclick = run
        button.disabled = busy
        if (!button.isConnected) actions.append(button)
      }
    }
  }
  refresh()
  return {
    refresh,
    destroy() {
      alive = false
      styleBinding.destroy()
      root.remove()
    },
  }
}

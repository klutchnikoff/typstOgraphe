import { tr } from './i18n'
import type { Catalogue, DocumentType, ModelPolicy } from './shell'
import type { Settings } from './state'
import type { MetadataCommands } from './metadata-commands'
import { bindMetadataInput } from './metadata-input'

/** General settings expose only options the active model lets the author change. */
export function documentOptions(
  parent: HTMLElement,
  kind: DocumentType,
  catalogue: Catalogue,
  settings: Settings,
  binding: { commands: MetadataCommands; read: () => Settings },
  policy: () => ModelPolicy | undefined = () => undefined,
) {
  const root = document.createElement('div')
  root.className = 'document-options'
  const left = document.createElement('div'),
    right = document.createElement('div')
  left.className = right.className = 'document-options-column'
  root.append(left, right)
  const choice = (
    column: HTMLElement,
    name: string,
    label: string,
    items: { value: string; label: string }[],
  ) => {
    const title = document.createElement('label')
    title.htmlFor = `document-option-${name}`
    title.textContent = label
    const control = document.createElement('div')
    control.className = 'document-option-control'
    const select = document.createElement('select')
    select.id = title.htmlFor
    for (const item of items) select.add(new Option(item.label, item.value))
    control.append(select)
    const field = document.createElement('div')
    field.className = 'field'
    field.style.display = 'contents'
    field.append(title, control)
    column.append(field)
    return { select, control, title }
  }
  const model = choice(
    left,
    'model',
    tr('Modèle'),
    kind.models.map((model) => ({ value: model.id, label: model.label })),
  )
  const paper = choice(left, 'paper', tr('Format'), catalogue.papers)
  const orientation = choice(left, 'orientation', tr('Orientation'), [
    { value: 'false', label: tr('Portrait') },
    { value: 'true', label: tr('Paysage') },
  ])
  const language = choice(right, 'language', tr('Langue du document'), catalogue.languages)
  const outlineRow = document.createElement('label'),
    outline = document.createElement('input')
  outline.type = 'checkbox'
  outline.setAttribute('aria-label', tr('Afficher le sommaire'))
  outlineRow.className = 'document-option-checkbox'
  outlineRow.append(outline, document.createTextNode(tr('Afficher le sommaire')))
  right.append(outlineRow)
  parent.append(root)
  const descriptor = kind.structured?.outline
  const outlineBinding = descriptor
    ? bindMetadataInput(
        outline,
        binding.commands,
        { kind: 'metadata', field: descriptor.field },
        () => binding.read().metadata.fields[descriptor.parameter],
        () => outline.checked,
      )
    : undefined
  const optionBindings = [
    bindMetadataInput(
      model.select,
      binding.commands,
      { kind: 'model' },
      () => ({ model_id: binding.read().modelId, source: binding.read().template }),
      () => {
        const selected = kind.models.find((m) => m.id === model.select.value)!
        return { model_id: selected.id, source: selected.source }
      },
      (value) => {
        const selected = value as { model_id: string; source: string }
        return (
          kind.models.find((m) => m.id === selected.model_id && m.source === selected.source)?.id ||
          'embedded'
        )
      },
    ),
    bindMetadataInput(
      paper.select,
      binding.commands,
      { kind: 'option', name: 'paper' },
      () => binding.read().metadata.fields.options.paper,
    ),
    bindMetadataInput(
      orientation.select,
      binding.commands,
      { kind: 'option', name: 'landscape' },
      () => binding.read().metadata.fields.options.landscape,
      () => orientation.select.value === 'true',
    ),
    bindMetadataInput(
      language.select,
      binding.commands,
      { kind: 'option', name: 'language' },
      () => binding.read().metadata.fields.options.language,
    ),
  ]
  const update = (current: Settings) => {
    // A saved project keeps its own model, even after the bundled model changes.
    const bundled = kind.models.find(
      (model) => model.id === current.modelId && model.source === current.template,
    )
    const embedded = [...model.select.options].find((option) => option.value === 'embedded')
    if (!bundled && !embedded) {
      const option = new Option(tr('Modèle embarqué'), 'embedded')
      option.disabled = true
      model.select.add(option)
    } else if (bundled) embedded?.remove()
    const page = bundled?.fixedPage
    const descriptor = kind.structured?.outline
    outlineRow.hidden = !descriptor || !policy()?.outline
    outlineBinding?.refresh()
    for (const control of [paper, orientation]) {
      control.title.hidden = control.control.hidden = !!page
      control.select.disabled = !!page
    }
    for (const binding of optionBindings) binding.refresh()
  }
  update(settings)
  return {
    update,
    destroy: () => {
      outlineBinding?.destroy()
      for (const binding of optionBindings) binding.destroy()
    },
  }
}

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
  changeModel: (id: string) => void,
  changeOption: (key: string, value: string | boolean) => void,
  changeField: (key: string, value: boolean) => void = () => {},
  policy: () => ModelPolicy | undefined = () => undefined,
  binding?: { commands: MetadataCommands; read: () => Settings },
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
    column.append(title, control)
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
  outline.onchange = () => {
    const descriptor = kind.structured?.outline
    if (descriptor) changeField(descriptor.field, outline.checked)
  }
  parent.append(root)
  const descriptor = kind.structured?.outline
  const outlineBinding =
    binding && descriptor
      ? bindMetadataInput(
          outline,
          binding.commands,
          { kind: 'metadata', field: descriptor.field },
          () => binding.read().metadata.fields[descriptor.parameter],
          () => outline.checked,
        )
      : undefined
  if (outlineBinding) outline.onchange = null
  model.select.onchange = () => changeModel(model.select.value)
  paper.select.onchange = () => changeOption('paper', paper.select.value)
  orientation.select.onchange = () => changeOption('landscape', orientation.select.value === 'true')
  language.select.onchange = () => changeOption('language', language.select.value)
  const update = (current: Settings) => {
    // A saved project keeps its own model, even after the bundled model changes.
    const bundled = kind.models.find(
      (model) => model.id === current.modelId && model.source === current.template,
    )
    const embedded = [...model.select.options].find((option) => option.value === 'embedded')
    if (!bundled && !embedded) model.select.add(new Option(tr('Modèle embarqué'), 'embedded'))
    else if (bundled) embedded?.remove()
    const selected = bundled ? bundled.id : 'embedded'
    if (model.select.value !== selected) model.select.value = selected
    const page = bundled?.fixedPage
    const descriptor = kind.structured?.outline
    outlineRow.hidden = !descriptor || !policy()?.outline
    if (outlineBinding) outlineBinding.refresh()
    else outline.checked = !!(descriptor && current.metadata.fields[descriptor.parameter])
    const options = current.metadata.fields.options
    for (const [control, value] of [
      [paper, String(options.paper)],
      [orientation, String(options.landscape)],
    ] as const) {
      if (control.select.value !== value) control.select.value = value
      control.title.hidden = control.control.hidden = !!page
      control.select.disabled = !!page
    }
    if (language.select.value !== String(options.language))
      language.select.value = String(options.language)
  }
  update(settings)
  return { update, destroy: () => outlineBinding?.destroy() }
}

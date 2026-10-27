import { tr } from './i18n'
import type { Catalogue, DocumentType } from './shell'
import type { Settings } from './state'

/** General settings expose only options the active model lets the author change. */
export function documentOptions(
  parent: HTMLElement,
  kind: DocumentType,
  catalogue: Catalogue,
  settings: Settings,
  changeModel: (id: string) => void,
  changeOption: (key: string, value: string | boolean) => void,
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
  parent.append(root)
  model.select.onchange = () => changeModel(model.select.value)
  paper.select.onchange = () => changeOption('paper', paper.select.value)
  orientation.select.onchange = () => changeOption('landscape', orientation.select.value === 'true')
  language.select.onchange = () => changeOption('language', language.select.value)
  const update = (current: Settings) => {
    // An embedded model may already belong to a reopened document, without an import action here.
    const embedded = [...model.select.options].find((option) => option.value === 'embedded')
    if (current.modelId === 'embedded' && !embedded)
      model.select.add(new Option(tr('Modèle embarqué'), 'embedded'))
    else if (current.modelId !== 'embedded') embedded?.remove()
    if (model.select.value !== current.modelId) model.select.value = current.modelId
    const page = kind.models.find((model) => model.id === current.modelId)?.fixedPage
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
  return { update }
}

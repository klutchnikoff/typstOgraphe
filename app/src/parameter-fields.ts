import { tr } from './i18n'
import type { SettingField, Scalar } from './capability-settings'

/** Fields are generated from the same schemas used to determine availability. */
export class ParameterFields {
  private controls = new Map<
    SettingField,
    HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
  >()
  private image?: { field: SettingField; button: HTMLButtonElement }
  private drafts: Record<string, Scalar | undefined> = {}
  constructor(
    private root: HTMLElement,
    private chooseImage: () => void,
  ) {}
  render(fields: SettingField[], drafts: Record<string, Scalar | undefined>) {
    this.drafts = drafts
    this.controls.clear()
    this.image = undefined
    this.root.replaceChildren()
    for (const field of fields) {
      if (drafts[field.name] === undefined) drafts[field.name] = field.schema.default
      if (field.control === 'image') {
        const row = document.createElement('div')
        row.id = 'image-options'
        const button = document.createElement('button')
        button.type = 'button'
        button.id = 'replace-image'
        button.textContent = tr('Remplacer…')
        button.onclick = this.chooseImage
        const hint = document.createElement('span')
        hint.id = 'image-help'
        hint.textContent = tr('La légende se modifie directement dans le bloc.')
        row.append(button, hint)
        this.root.append(row)
        this.image = { field, button }
        continue
      }
      const input =
        field.control === 'textarea'
          ? document.createElement('textarea')
          : field.control === 'select'
            ? document.createElement('select')
            : document.createElement('input')
      input.id = `parameter-${field.name}`
      input.setAttribute('aria-label', field.label)
      // JSON Schema requires a boolean value to be present, not necessarily true.
      input.required = field.required && field.control !== 'checkbox'
      if (input instanceof HTMLInputElement) {
        input.type =
          field.control === 'checkbox' ? 'checkbox' : field.control === 'number' ? 'number' : 'text'
        if (field.control === 'number') {
          input.step = field.schema.type === 'integer' ? '1' : 'any'
          if (field.schema.minimum !== undefined) input.min = String(field.schema.minimum)
          if (field.schema.maximum !== undefined) input.max = String(field.schema.maximum)
        }
        if (field.schema.pattern) input.pattern = field.schema.pattern
      }
      if (!(input instanceof HTMLSelectElement)) {
        if (field.schema.minLength !== undefined) input.minLength = field.schema.minLength
        if (field.schema.maxLength !== undefined) input.maxLength = field.schema.maxLength
      } else {
        if (field.schema.default === undefined) input.add(new Option('—', ''))
        for (const value of field.schema.enum || [])
          input.add(new Option(String(value), JSON.stringify(value)))
      }
      if (field.control === 'checkbox') (input as HTMLInputElement).checked = !!drafts[field.name]
      else
        input.value =
          input instanceof HTMLSelectElement
            ? drafts[field.name] === undefined
              ? ''
              : JSON.stringify(drafts[field.name])
            : String(drafts[field.name] ?? '')
      const read = () =>
        field.control === 'checkbox'
          ? (input as HTMLInputElement).checked
          : input instanceof HTMLSelectElement
            ? input.value === ''
              ? undefined
              : (JSON.parse(input.value) as Scalar)
            : field.control === 'number'
              ? input.value === ''
                ? field.schema.default
                : Number(input.value)
              : input.value
      input.oninput = input.onchange = () => {
        drafts[field.name] = read()
      }
      const label = document.createElement('label')
      label.htmlFor = input.id
      label.textContent = field.label
      label.hidden = field.schema['x-editor']?.label_visibility === 'accessible'
      this.root.append(label, input)
      this.controls.set(field, input)
    }
  }
  read() {
    for (const [field, input] of this.controls) {
      if (!input.checkValidity()) {
        input.focus()
        return { error: `${tr('Vérifiez la valeur de ce champ.')} ${field.label}` }
      }
      input.dispatchEvent(new Event('input'))
    }
    const attrs: Record<string, Scalar | undefined> = {}
    for (const field of [...this.controls.keys(), ...(this.image ? [this.image.field] : [])])
      attrs[field.attribute] = this.drafts[field.name]
    return { attrs }
  }
  replaceImage(path: string) {
    if (!this.image) return false
    this.drafts[this.image.field.name] = path
    this.image.button.textContent = tr('Image remplacée — appliquer pour confirmer')
    this.image.button.focus()
    return true
  }
}

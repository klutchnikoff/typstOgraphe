/** Shared accessible label/control frame for document and detached forms. */
export function labeledField(
  label: string,
  input: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
  controls?: HTMLElement,
) {
  const row = document.createElement('div')
  row.className = 'field'
  const caption = document.createElement('label')
  caption.textContent = label
  caption.htmlFor = input.id
  if (!controls) {
    controls = document.createElement('div')
    controls.className = 'metadata-field-controls'
    controls.append(input)
  }
  row.append(caption, controls)
  return row
}

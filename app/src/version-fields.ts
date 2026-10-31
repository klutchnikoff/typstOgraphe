import { tr } from './i18n'
import type { VersionDetails } from './version-types'
/** Same fields for creation and relabelling; immutable metadata never passes through this form. */
export function versionFields(details?: VersionDetails) {
  const element = document.createElement('div')
  element.className = 'version-fields'
  function field(name: string, label: string, multiline = false) {
    const root = document.createElement('label')
    const title = document.createElement('span')
    title.textContent = tr(label)
    const input = document.createElement(multiline ? 'textarea' : 'input')
    input.name = name
    root.append(title, input)
    element.append(root)
    return input
  }
  const name = field('name', 'Nom de la version')
  name.required = !!details
  name.maxLength = 160
  name.value = details?.name || ''
  name.placeholder = tr('Nom daté automatique')
  const tags = field('tags', 'Tags séparés par des virgules')
  tags.value = details?.tags.join(', ') || ''
  const description = field('description', 'Description facultative', true)
  description.value = details?.description || ''
  description.maxLength = 16384
  return {
    element,
    valid: () => name.reportValidity(),
    focus: () => name.focus(),
    value(): VersionDetails {
      return {
        name: name.value.trim(),
        tags: [
          ...new Set(
            tags.value
              .split(',')
              .map((tag) => tag.trim())
              .filter(Boolean),
          ),
        ],
        description: description.value || null,
        creator: details?.creator || null,
      }
    },
  }
}

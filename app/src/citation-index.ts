import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import { documentType, settingsFor } from './state'
export interface ReferenceFile {
  id: string
  name: string
  contents: string
}
export interface ReferenceEntry {
  file_id: string
  file_name: string
  key: string
  title: string
  authors: string
  year: string
  label: string
}
export interface CitationItem {
  file_id: string
  key: string
  supplement: string
}
export interface Citation {
  items: CitationItem[]
  form: 'normal' | 'prose'
}
const indexes = new WeakMap<Editor, ReferenceEntry[]>()
export function referenceFiles(provider: Provider): ReferenceFile[] {
  const descriptor = documentType(provider).structured?.bibliography
  return descriptor
    ? (settingsFor(provider).metadata.fields[descriptor.files] as ReferenceFile[]) || []
    : []
}
export function setCitationEntries(editor: Editor, entries: ReferenceEntry[]) {
  indexes.set(editor, entries)
  editor.view.dispatch(editor.state.tr.setMeta('bibliography-index', true))
}
export function citationLabel(
  editor: Editor,
  items: CitationItem[],
  form: Citation['form'] = 'normal',
) {
  return formatCitationLabel(indexes.get(editor) || [], items, form)
}
export function formatCitationLabel(
  entries: ReferenceEntry[],
  items: CitationItem[],
  form: Citation['form'] = 'normal',
) {
  const label = items
    .map(
      (item) =>
        entries.find((entry) => entry.file_id === item.file_id && entry.key === item.key)?.label ||
        item.key,
    )
    .join(' ; ')
  return label && form === 'normal' ? `(${label})` : label
}

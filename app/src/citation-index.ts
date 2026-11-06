import type { Editor } from '@tiptap/core'
import type { Provider } from './provider'
import { documentType, settingsFor } from './state'
export interface BibliographicRecord {
  id: string
  key: string
  entry: Record<string, unknown>
  retained_parents?: Record<string, unknown>[]
  provenance: unknown[]
}
export interface ReferenceEntry {
  record_id: string
  key: string
  title: string
  authors: string
  year: string
  label: string
}
export interface CitationItem {
  record_id: string
  supplement: string
}
export interface Citation {
  items: CitationItem[]
  form: 'normal' | 'prose'
}
const indexes = new WeakMap<Editor, ReferenceEntry[]>()
export function referenceRecords(provider: Provider): BibliographicRecord[] {
  const descriptor = documentType(provider).structured?.bibliography
  return descriptor
    ? (settingsFor(provider).metadata.fields[descriptor.files] as BibliographicRecord[]) || []
    : []
}
export function citationEntry(editor: Editor, id: string) {
  return indexes.get(editor)?.find((entry) => entry.record_id === id)
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
    .map((item) => entries.find((entry) => entry.record_id === item.record_id)?.label || '?')
    .join(' ; ')
  return label && form === 'normal' ? `(${label})` : label
}

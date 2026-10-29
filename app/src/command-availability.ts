import type { Editor } from '@tiptap/core'
import { getMarkRange } from '@tiptap/core'
import { AllSelection, NodeSelection } from '@tiptap/pm/state'
import { CellSelection } from '@tiptap/pm/tables'
import { editingProvider } from './editing-history'
import { editingView } from './editing-context'
import { commandCapabilities, hasCommandTarget } from './command-context'
import { tr } from './i18n'

export type InteractionPolicy = 'style' | 'link' | 'heading' | 'block' | 'list' | 'inline' | 'table'
export interface Availability {
  enabled: boolean
  reason?: string
}
export const allowed: Availability = { enabled: true }
export const refused = (reason: string): Availability => ({ enabled: false, reason })

const reasons: Record<string, string> = {
  no_selection: 'Placez le curseur dans une zone de rédaction.',
  unsupported_document_type: 'Cette commande n’est pas disponible pour ce type de document.',
  text_required: 'Cette commande nécessite une zone de texte compatible.',
  selection_required: 'Sélectionnez du texte ou placez le curseur dans un lien.',
  body_required: 'Cette commande est réservée au corps du document.',
  top_level_required: 'Cette commande n’est pas disponible à l’intérieur de ce widget.',
  caret_required: 'Placez le curseur sans sélectionner de texte.',
  read_only: 'Cette zone n’est pas modifiable.',
  composing: 'Terminez la composition en cours.',
  disconnected: 'Reconnectez ce document avant d’appliquer une commande.',
  pending_draft: 'Conservez ou abandonnez le brouillon refusé avant de continuer.',
  no_document: 'Ouvrez ou créez un document.',
  modal: 'Terminez ou annulez le formulaire ouvert.',
  unavailable: 'Cette commande n’est pas disponible dans ce contexte.',
}
export function availabilityLabel(value: Availability) {
  return value.reason ? tr(reasons[value.reason] || value.reason) : ''
}
export function editingAvailability(editor: Editor, needsTarget = true): Availability {
  if (editor.isDestroyed || !editor.isEditable) return refused('read_only')
  if (needsTarget && !hasCommandTarget(editor)) return refused('no_selection')
  const provider = editingProvider(editor)
  if (editor.view.composing || provider?.composing) return refused('composing')
  if (provider?.rejected) return refused('pending_draft')
  if (provider?.nativeEditing && !provider.online) return refused('disconnected')
  return allowed
}
/** Immediate projected feedback follows the policy exported by the Rust catalogue.
 * Matching confirmed context replies refine it without another IPC or checkpoint.
 * No form opening is a promise that its future parameters will validate.
 */
export function interactionAvailability(editor: Editor, policy: InteractionPolicy): Availability {
  const editing = editingAvailability(editor)
  if (!editing.enabled) return editing
  const s = editor.state.selection
  const body = editor.view.dom.dataset.richZoneProfile === 'document'
  const text =
    s instanceof AllSelection ||
    s instanceof CellSelection ||
    (s.$from.parent.inlineContent && s.$to.parent.inlineContent)
  if (policy === 'style' || policy === 'link') {
    if (!text) return refused('text_required')
    if (policy === 'link' && s.empty && !getMarkRange(s.$from, editor.schema.marks.link))
      return refused('selection_required')
    return allowed
  }
  if (!body) return refused('body_required')
  const top = [s.$from, s.$to].every(($p) => $p.depth === 0 || $p.depth === 1)
  const firstIndex = s instanceof AllSelection ? 0 : s.$from.index(0)
  const lastIndex = s instanceof AllSelection ? editor.state.doc.childCount - 1 : s.$to.index(0)
  const paragraphs =
    top &&
    !(s instanceof NodeSelection) &&
    editor.state.doc.content.content
      .slice(firstIndex, lastIndex + 1)
      .every((node) => ['paragraph', 'heading'].includes(node.type.name))
  if (policy === 'inline')
    return text &&
      s.$from.sameParent(s.$to) &&
      !(s instanceof CellSelection) &&
      !(s instanceof AllSelection)
      ? allowed
      : refused('text_required')
  if (policy === 'list') {
    const first = editor.state.doc.maybeChild(s.$from.index(0))
    if (
      s.$from.index(0) === s.$to.index(0) &&
      ['bulletList', 'orderedList', 'descriptionList'].includes(first?.type.name || '')
    )
      return allowed
  }
  if ((policy === 'heading' || policy === 'table') && !paragraphs)
    return refused('top_level_required')
  if (policy === 'table' && !s.empty) return refused('caret_required')
  if (!top) return refused('top_level_required')
  if (s instanceof NodeSelection && policy === 'heading') return refused('text_required')
  return allowed
}
export function capabilityAvailability(editor: Editor, id: string, policy: InteractionPolicy) {
  const catalogue = commandCapabilities(editor)
  if (catalogue?.available()?.includes(id) === false) return refused('unsupported_document_type')
  policy = catalogue?.entries.find((c) => c.id === id)?.editor?.availability || policy
  const local = interactionAvailability(editor, policy)
  if (!local.enabled) return local
  const provider = editingProvider(editor),
    snapshot = provider?.historyState
  if (
    provider?.hasConfirmedContext(editor) &&
    snapshot?.guard?.document === provider.generation &&
    snapshot.guard.revision === provider.revision &&
    JSON.stringify(snapshot.selection) === JSON.stringify(editingView(editor)?.selection)
  )
    return snapshot.availability?.[id] || local
  return local
}

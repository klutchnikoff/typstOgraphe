import type { Provider } from './provider'
import type { FormDraft } from './private-drafts'
import { tr } from './i18n'
export interface RecoveredItem {
  id: string
  kind: 'form' | 'field' | 'body'
  payload: any
}
export const RECOVERED_CHANGED = 'recovery:items-changed'
export function notifyRecovered(provider: Provider) {
  document.dispatchEvent(new CustomEvent(RECOVERED_CHANGED, { detail: provider }))
}
/** Removing the queue item is acknowledged only after its new confirmed base is durable. */
export async function consumeRecovered(provider: Provider, id: string) {
  const index = provider.recoveredItems.findIndex((item) => item.id === id)
  if (index < 0) return
  const [item] = provider.recoveredItems.splice(index, 1)
  notifyRecovered(provider)
  try {
    await provider.privateRecovery?.force()
  } catch (error) {
    provider.recoveredItems.splice(index, 0, item)
    notifyRecovered(provider)
    throw error
  }
}
export function recoveredLabel(item: RecoveredItem) {
  if (item.kind === 'body') return tr('Texte non appliqué')
  if (item.kind === 'field') return tr('Champ d’en-tête')
  const form = item.payload as FormDraft,
    context = form.context as any
  if (context?.kind === 'record')
    return context.collection === 'authors'
      ? tr('Auteur')
      : context.collection === 'affiliations'
        ? tr('Affiliation')
        : tr('Élément d’en-tête')
  const kind = (form.data as any)?.data?.kind
  const names: Record<string, string> = {
    footnote: 'Note de bas de page',
    figure: 'Image',
    link: 'Lien',
    tableFigure: 'Tableau',
    bulletList: 'Liste à puces',
    orderedList: 'Liste numérotée',
    descriptionList: 'Liste de descriptions',
    citation: 'Citation',
  }
  return tr(names[kind] || (form.form === 'citation-properties' ? 'Citation' : 'Saisie en cours'))
}
export function readableDraft(value: unknown): string {
  if (value == null) return ''
  if (typeof value === 'string') return value
  if (Array.isArray(value)) return value.map(readableDraft).filter(Boolean).join('\n')
  if (typeof value === 'object') {
    const obj = value as Record<string, unknown>
    if (typeof obj.text === 'string') return obj.text
    if (Array.isArray(obj.content)) return readableDraft(obj.content)
    return Object.entries(obj)
      .filter(
        ([key]) =>
          !['update', 'resources', 'data', 'epoch', 'generation', 'sequence'].includes(key),
      )
      .map(([key, val]) => `${key}: ${readableDraft(val)}`)
      .join('\n')
  }
  return String(value)
}

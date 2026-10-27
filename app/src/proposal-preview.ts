import { generateHTML, type JSONContent } from '@tiptap/core'
import { richZoneExtensions } from './rich-zone'
import { tr } from './i18n'

/** Read-only semantic content; no editable views or widget controls. */
export function proposalPreview(node: JSONContent, imageURL: (path: string) => string) {
  const result = document.createElement('div')
  result.className = 'proposal-text ProseMirror'
  result.innerHTML = generateHTML(node, richZoneExtensions('document', { imageURL }))
  for (const note of result.querySelectorAll<HTMLElement>('[data-footnote]'))
    note.textContent = `${tr('Note de bas de page')} : ${note.dataset.footnote || ''}`
  result.addEventListener('click', (event) => {
    if ((event.target as Element).closest('a')) event.preventDefault()
  })
  return result
}
export function proposalDetails(before: JSONContent, after: JSONContent): string[] {
  const rows = new Set<string>()
  const labels: Record<string, string> = {
    text: 'Note de bas de page',
    alt: 'Texte alternatif',
    src: 'Image',
    href: 'Lien',
    start: 'Premier numéro',
    level: 'Niveau de titre',
  }
  const walk = (a: JSONContent, b: JSONContent) => {
    if (a.type !== b.type) rows.add(tr('Structure modifiée'))
    if (JSON.stringify(a.marks || []) !== JSON.stringify(b.marks || [])) {
      rows.add(tr('Mise en forme modifiée'))
      const link = (n: JSONContent) => n.marks?.find((m) => m.type === 'link')?.attrs?.href || ''
      if (link(a) !== link(b)) rows.add(`${tr('Lien')} : ${link(a)} → ${link(b)}`)
    }
    for (const key of new Set([...Object.keys(a.attrs || {}), ...Object.keys(b.attrs || {})]))
      if (JSON.stringify(a.attrs?.[key]) !== JSON.stringify(b.attrs?.[key])) {
        const label = labels[key]
        rows.add(
          label
            ? `${tr(label)} : ${String(a.attrs?.[key] ?? '')} → ${String(b.attrs?.[key] ?? '')}`
            : tr('Paramètres modifiés'),
        )
      }
    const left = a.content || [],
      right = b.content || []
    if (left.length !== right.length) rows.add(tr('Structure modifiée'))
    for (let i = 0; i < Math.min(left.length, right.length); i++) walk(left[i], right[i])
  }
  walk(before, after)
  return [...rows]
}

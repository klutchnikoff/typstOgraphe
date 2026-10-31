import type { Transport } from './provider'
import { tr } from './i18n'
import { icon } from './icons'
import { formDialog } from './form-dialog'
import { navigationGroup } from './keyboard-navigation'
export interface WorkEntry {
  id: string
  name: string
  documentType: string
  documentTypeLabel: string
  savedAt: number
  window: string
  originalPath: string | null
  pending: number
  error: string | null
}
export async function workList(
  content: HTMLElement,
  transport: Transport,
  resume: (entry: WorkEntry) => void,
  failed: (error: unknown) => void,
) {
  const body = document.createElement('div')
  body.className = 'recovery-work-list'
  content.replaceChildren(body)
  navigationGroup(body, { axis: 'vertical', items: '.recovery-work-row' })
  const fill = async () => {
    const entries = await transport.invoke<WorkEntry[]>('work_list')
    if (!body.isConnected) return
    body.replaceChildren()
    content.hidden = entries.length === 0
    const groups = new Map<string, WorkEntry[]>()
    for (const entry of entries) {
      const key = entry.originalPath || entry.id.split('/').slice(0, 2).join('/')
      const group = groups.get(key) || []
      group.push(entry)
      groups.set(key, group)
    }
    for (const group of groups.values()) {
      const article = document.createElement('article')
      const heading = document.createElement('h3')
      heading.textContent = group[0].name
      article.append(heading)
      for (const [index, entry] of group.entries()) {
        const row = document.createElement('div')
        row.className = 'recovery-work-row'
        row.tabIndex = 0
        row.dataset.recoveryId = entry.id
        const detail = document.createElement('div'),
          date = document.createElement('p'),
          state = document.createElement('p')
        date.textContent = entry.savedAt
          ? new Date(entry.savedAt * 1000).toLocaleString(
              document.documentElement.lang || undefined,
            )
          : ''
        state.textContent = entry.error
          ? tr('Cette copie ne peut pas être lue.')
          : entry.pending
            ? `${entry.pending} · ${tr('Saisies à reprendre')}`
            : tr('Modifications non enregistrées')
        detail.append(date, state)
        if (group.length > 1)
          date.textContent = `${tr('Fenêtre')} ${index + 1} · ${date.textContent}`
        if (entry.documentType)
          date.textContent = `${entry.documentTypeLabel || entry.documentType} · ${date.textContent}`
        const actions = document.createElement('div')
        actions.className = 'recovery-work-actions'
        const remove = document.createElement('button'),
          accept = document.createElement('button')
        remove.type = accept.type = 'button'
        remove.className = 'icon-button danger'
        remove.innerHTML = icon('trash-2')
        remove.setAttribute('aria-label', tr('Supprimer la copie'))
        accept.className = 'primary'
        accept.textContent = tr('Reprendre')
        accept.disabled = !!entry.error
        accept.onclick = () => resume(entry)
        remove.onclick = () => {
          let busy = false
          const close = () => {
            if (!busy) {
              form.destroy()
              remove.focus()
            }
          }
          const form = formDialog({
            id: `delete-recovery-${crypto.randomUUID()}`,
            title: tr('Supprimer cette copie de secours ?'),
            cancel: close,
            returnFocus: () => remove,
          })
          const text = document.createElement('p')
          text.textContent = tr(
            'Son contenu non enregistré ne pourra plus être repris. Le fichier d’origine restera inchangé.',
          )
          form.body.append(text)
          form.action({
            label: tr('Supprimer la copie'),
            danger: true,
            run: () => {
              if (busy) return
              busy = true
              void transport
                .invoke('work_delete', { id: entry.id })
                .then(async () => {
                  form.destroy()
                  await fill()
                })
                .catch((error) => {
                  busy = false
                  form.error.textContent = String(error)
                })
            },
          })
          form.action({ label: tr('Annuler'), primary: true, run: close })
          form.open()
        }
        actions.append(remove, accept)
        row.append(detail, actions)
        article.append(row)
        row.addEventListener('keydown', (event) => {
          if (event.target === row && event.key === 'Enter') {
            event.preventDefault()
            accept.click()
          }
        })
      }
      body.append(article)
    }
  }
  await fill().catch(failed)
  return { refresh: fill }
}

import { tr } from './i18n'
import type { Transport } from './provider'

interface Entry {
  id: string
  name: string
  savedAt: number
  error: string | null
}
export async function recoveryPanel(
  content: HTMLElement,
  transport: Transport,
  restore: (id: string) => void,
  failed: (error: unknown) => void,
) {
  const body = document.createElement('div')
  content.replaceChildren(body)
  const fill = async () => {
    const entries = await transport.invoke<Entry[]>('recovery_list')
    if (content.firstChild !== body) return
    body.replaceChildren()
    if (!entries.length) {
      body.textContent = tr('Aucune copie à récupérer.')
      return
    }
    for (const entry of entries) {
      const row = document.createElement('section')
      const title = document.createElement('p')
      title.textContent = entry.name
      const detail = document.createElement('p')
      detail.textContent =
        entry.error ||
        (entry.savedAt
          ? new Date(entry.savedAt * 1000).toLocaleString(
              document.documentElement.lang || undefined,
            )
          : '')
      const actions = document.createElement('div')
      actions.className = 'panel-actions'
      const remove = document.createElement('button')
      remove.className = 'danger'
      remove.textContent = tr('Supprimer la copie')
      remove.onclick = () => {
        void transport.invoke('recovery_delete', { id: entry.id }).then(fill).catch(failed)
      }
      const accept = document.createElement('button')
      accept.className = 'primary'
      accept.textContent = tr('Récupérer')
      accept.disabled = !!entry.error
      accept.onclick = () => restore(entry.id)
      actions.append(remove, accept)
      row.append(title, detail, actions)
      body.append(row)
    }
  }
  await fill()
}

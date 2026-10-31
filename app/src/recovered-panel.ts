import type { Provider } from './provider'
import {
  consumeRecovered,
  recoveredLabel,
  readableDraft,
  RECOVERED_CHANGED,
  type RecoveredItem,
} from './recovered-work'
import { icon } from './icons'
import { formDialog } from './form-dialog'
import { navigationGroup } from './keyboard-navigation'
import { tr } from './i18n'
/** Temporary queue. Reusing a form never consumes the durable source merely on open/cancel. */
export function recoveredPanel(
  content: HTMLElement,
  provider: Provider,
  reopen: (item: RecoveredItem) => boolean,
  native: boolean,
  itemLabel: (item: RecoveredItem) => string = recoveredLabel,
) {
  const root = document.createElement('section')
  root.className = 'recovered-items'
  content.replaceChildren(root)
  const list = document.createElement('div')
  navigationGroup(list, { axis: 'vertical', items: ':scope > section' })
  const refresh = () => {
    if (!root.isConnected) return
    const focused = document.activeElement as HTMLElement
    const heading = document.createElement('p')
    heading.className = 'hint'
    heading.textContent = tr('Chaque saisie reste privée jusqu’à sa validation.')
    list.replaceChildren()
    root.replaceChildren(heading, list)
    if (!provider.recoveredItems.length) {
      const empty = document.createElement('p')
      empty.textContent = tr('Toutes les saisies ont été traitées.')
      list.append(empty)
    }
    for (const item of provider.recoveredItems) {
      const row = document.createElement('section')
      row.tabIndex = 0
      const title = document.createElement('h3')
      title.textContent = itemLabel(item)
      const detail = document.createElement('p')
      detail.className = 'hint'
      detail.textContent =
        item.kind === 'body'
          ? tr('Cette saisie reste séparée du document.')
          : tr('Saisie récupérée · à valider')
      const actions = document.createElement('div')
      actions.className = 'panel-actions'
      const remove = document.createElement('button'),
        open = document.createElement('button')
      remove.type = open.type = 'button'
      remove.className = 'danger icon-button'
      remove.innerHTML = icon('trash-2')
      remove.setAttribute('aria-label', tr('Abandonner cette saisie'))
      open.textContent = tr(item.kind === 'body' ? 'Consulter' : 'Reprendre la saisie')
      open.dataset.recoveredId = item.id
      open.onclick = () => {
        try {
          if (item.kind === 'body' || !reopen(item)) isolated(item, open)
        } catch {
          isolated(item, open)
        }
      }
      row.addEventListener('keydown', (event) => {
        if (event.target === row && event.key === 'Enter') {
          event.preventDefault()
          open.click()
        }
      })
      remove.onclick = () => {
        const close = () => {
          form.destroy()
          open.focus()
        }
        const form = formDialog({
          id: 'discard-recovered-item',
          title: tr('Abandonner cette saisie ?'),
          cancel: close,
          returnFocus: () => open,
        })
        form.body.textContent = tr('Le contenu actuel du document restera inchangé.')
        form.action({
          label: tr('Abandonner la saisie'),
          danger: true,
          run: () => {
            void consumeRecovered(provider, item.id)
              .then(() => {
                form.destroy()
                refresh()
              })
              .catch((error) => (form.error.textContent = String(error)))
          },
        })
        form.action({ label: tr('Annuler'), primary: true, run: close })
        form.open()
      }
      actions.append(remove, open)
      row.append(title, detail, actions)
      list.append(row)
    }
    if (focused?.dataset.recoveredId)
      root
        .querySelector<HTMLElement>(
          `[data-recovered-id="${CSS.escape(focused.dataset.recoveredId)}"]`,
        )
        ?.focus({ preventScroll: true })
  }
  function isolated(item: RecoveredItem, trigger: HTMLElement) {
    const close = () => {
      form.destroy()
      trigger.focus()
    }
    const form = formDialog({
      id: 'isolated-recovered-item',
      title: itemLabel(item),
      cancel: close,
      returnFocus: () => trigger,
    })
    const explanation = document.createElement('p')
    explanation.textContent = tr(
      item.kind === 'body'
        ? 'Cette saisie reste séparée du document.'
        : 'La cible a changé ou n’est plus disponible. Votre saisie est conservée.',
    )
    const label = document.createElement('label')
    label.textContent = tr('Contenu récupéré')
    label.className = 'field'
    const text = document.createElement('textarea')
    text.readOnly = true
    text.rows = 9
    text.setAttribute('aria-label', tr('Contenu récupéré'))
    const payload = item.payload
    text.value =
      item.kind === 'body'
        ? readableDraft(payload.input?.document || payload.content?.document || payload)
        : item.kind === 'field'
          ? readableDraft(payload.value)
          : [
              readableDraft(payload.data?.fields?.map((field: any) => field.value)),
              readableDraft(payload.data?.rich?.map((field: any) => field.document)),
              readableDraft(payload.data?.data),
            ]
              .filter(Boolean)
              .join('\n\n')
    label.append(text)
    form.body.append(explanation, label)
    if (native)
      form.action({
        label: tr('Exporter le brouillon…'),
        run: () => {
          void provider.transport
            .invoke<string | null>('choose_work_file', { save: true })
            .then((path) =>
              path ? provider.transport.invoke('work_export', { id: item.id, path }) : undefined,
            )
            .catch((error) => (form.error.textContent = String(error)))
        },
      })
    form.action({ label: tr('Fermer'), primary: true, run: close })
    form.open()
    text.focus()
  }
  const changed = (event: Event) => {
    if ((event as CustomEvent).detail === provider) refresh()
  }
  document.addEventListener(RECOVERED_CHANGED, changed)
  refresh()
  return () => document.removeEventListener(RECOVERED_CHANGED, changed)
}

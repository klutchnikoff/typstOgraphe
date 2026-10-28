import { actionRow, composing, navigationGroup } from './keyboard-navigation'
import type { Editor } from '@tiptap/core'
import { tr } from './i18n'
import { searchState } from './proofing'
import { proofingKey } from './proofing'
import { replaceText } from './text-edits'

export function navigateSearch(editor: Editor, backwards = false, focus = true) {
  const hits = proofingKey.getState(editor.state)?.hits || []
  const { from, to, empty } = editor.state.selection
  const hit = backwards
    ? [...hits].reverse().find((h) => h.to <= from && (empty || h.from !== from)) || hits.at(-1)
    : hits.find((h) => h.from >= to && (empty || h.from !== from)) || hits[0]
  if (!hit) return false
  const chain = editor.chain().setTextSelection({ from: hit.from, to: hit.to }).scrollIntoView()
  if (focus) chain.focus()
  return chain.run()
}

export function searchPanel(content: HTMLElement, editor: Editor) {
  const query = document.createElement('input')
  query.type = 'search'
  query.placeholder = tr('Rechercher')
  query.setAttribute('aria-label', tr('Rechercher'))
  const replacement = document.createElement('input')
  replacement.placeholder = tr('Remplacer par')
  replacement.setAttribute('aria-label', tr('Remplacer par'))
  let lastDocument = editor.state.doc
  const results = document.createElement('div')
  const selectMatch = (button: HTMLElement) => {
    const from = Number(button.dataset.from),
      to = Number(button.dataset.to)
    if (matches().some((hit) => hit.from === from && hit.to === to))
      editor.chain().setTextSelection({ from, to }).scrollIntoView().run()
  }
  const navigation = navigationGroup(results, {
    axis: 'vertical',
    input: query,
    initial: false,
    beforeNavigate: () => {
      if (lastDocument !== editor.state.doc) fill()
    },
    enter: false,
    onSelect: selectMatch,
  })
  const insensitive = document.createElement('input')
  insensitive.type = 'checkbox'
  insensitive.checked = true
  const label = document.createElement('label')
  label.append(insensitive, document.createTextNode(tr(' Ignorer la casse')))
  const matches = () => searchState(editor, query.value, insensitive.checked)
  let pending = false
  const error = document.createElement('p')
  error.setAttribute('role', 'alert')
  const replace = async (hits: ReturnType<typeof matches>) => {
    if (pending || !hits.length) return
    const text = replacement.value,
      originalQuery = query.value,
      ignoreCase = insensitive.checked
    const currentPanel = () =>
      query.isConnected &&
      content.contains(query) &&
      !content.closest('[hidden]') &&
      query.value === originalQuery &&
      replacement.value === text &&
      insensitive.checked === ignoreCase
    pending = true
    current.disabled = all.disabled = true
    error.textContent = ''
    try {
      await replaceText(
        editor,
        hits.map((hit) => ({ ...hit, text })),
        {
          focus: false,
          current: currentPanel,
        },
      )
      if (currentPanel()) fill()
    } catch (reason) {
      if (currentPanel())
        error.textContent = reason instanceof Error ? reason.message : String(reason)
    } finally {
      pending = false
      current.disabled = all.disabled = false
    }
  }
  const fill = () => {
    lastDocument = editor.state.doc
    results.replaceChildren()
    for (const match of matches()) {
      const button = document.createElement('button')
      button.type = 'button'
      button.dataset.from = String(match.from)
      button.dataset.to = String(match.to)
      button.textContent = match.word
      results.append(button)
    }
    navigation.refresh(true)
  }
  const current = document.createElement('button')
  current.textContent = tr('Remplacer')
  current.onclick = () => {
    const { from, to } = editor.state.selection
    const match = matches().find((hit) => hit.from === from && hit.to === to)
    if (!match) return
    void replace([match])
  }
  const all = document.createElement('button')
  all.textContent = tr('Tout remplacer')
  all.className = 'danger'
  all.onclick = () => {
    void replace(matches())
  }
  query.oninput = fill
  query.onkeydown = (event) => {
    if (
      event.key === 'Enter' &&
      !composing(event) &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey
    ) {
      event.preventDefault()
      if (lastDocument !== editor.state.doc) fill()
      navigateSearch(editor, event.shiftKey, false)
      navigation.select(
        [...results.children].find(
          (button) => Number((button as HTMLElement).dataset.from) === editor.state.selection.from,
        ) as HTMLElement | undefined,
      )
    }
  }
  insensitive.onchange = fill
  const actions = actionRow()
  actions.append(all, current)
  const previous = document.createElement('button')
  previous.textContent = tr('Occurrence précédente')
  previous.onclick = () => navigateSearch(editor, true, false)
  const next = document.createElement('button')
  next.textContent = tr('Occurrence suivante')
  next.onclick = () => navigateSearch(editor, false, false)
  actions.prepend(previous, next)
  content.append(query, replacement, label, actions, error, results)
  query.focus()
}

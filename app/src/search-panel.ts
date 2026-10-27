import type { Editor } from '@tiptap/core'
import { tr } from './i18n'
import { searchState } from './proofing'
import { proofingKey } from './proofing'

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
  const results = document.createElement('div')
  const insensitive = document.createElement('input')
  insensitive.type = 'checkbox'
  insensitive.checked = true
  const label = document.createElement('label')
  label.append(insensitive, document.createTextNode(tr(' Ignorer la casse')))
  const matches = () => searchState(editor, query.value, insensitive.checked)
  const fill = () => {
    results.replaceChildren()
    for (const match of matches()) {
      const button = document.createElement('button')
      button.textContent = match.word
      button.onclick = () =>
        editor
          .chain()
          .focus()
          .setTextSelection({ from: match.from, to: match.to })
          .scrollIntoView()
          .run()
      results.append(button)
    }
  }
  const replacementContent = () =>
    replacement.value ? { type: 'text', text: replacement.value } : []
  const current = document.createElement('button')
  current.textContent = tr('Remplacer')
  current.onclick = () => {
    const { from, to } = editor.state.selection
    const match = matches().find((hit) => hit.from === from && hit.to === to)
    if (!match) return
    editor.chain().focus().insertContentAt({ from, to }, replacementContent()).run()
    fill()
  }
  const all = document.createElement('button')
  all.textContent = tr('Tout remplacer')
  all.className = 'danger'
  all.onclick = () => {
    const chain = editor.chain()
    for (const match of matches().reverse())
      chain.insertContentAt({ from: match.from, to: match.to }, replacementContent())
    chain.run()
    fill()
  }
  query.oninput = fill
  query.onkeydown = (event) => {
    if (event.key === 'Enter' && !event.isComposing) {
      event.preventDefault()
      navigateSearch(editor, event.shiftKey, false)
    }
  }
  insensitive.onchange = fill
  const actions = document.createElement('div')
  actions.className = 'panel-actions'
  actions.append(all, current)
  const previous = document.createElement('button')
  previous.textContent = tr('Occurrence précédente')
  previous.onclick = () => navigateSearch(editor, true)
  const next = document.createElement('button')
  next.textContent = tr('Occurrence suivante')
  next.onclick = () => navigateSearch(editor)
  actions.prepend(previous, next)
  content.append(query, replacement, label, actions, results)
  query.focus()
}

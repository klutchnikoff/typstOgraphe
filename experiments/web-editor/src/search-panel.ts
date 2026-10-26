import type { Editor } from '@tiptap/core'
import { tr } from './i18n'
import { searchState } from './proofing'

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
  all.onclick = () => {
    const chain = editor.chain()
    for (const match of matches().reverse())
      chain.insertContentAt({ from: match.from, to: match.to }, replacementContent())
    chain.run()
    fill()
  }
  query.oninput = fill
  insensitive.onchange = fill
  content.append(query, replacement, label, current, all, results)
  query.focus()
}

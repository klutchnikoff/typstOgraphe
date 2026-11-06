import { openBibliography } from './bibliography-manager'
import { focusEditorSelection } from './editor-focus'
import { editingAvailability } from './command-availability'
import { editingProvider } from './editing-history'
import type { Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { TextSelection } from '@tiptap/pm/state'
import { contextPanel } from './context-panel'
import { navigationGroup, actionRow } from './keyboard-navigation'
import type { BandHost } from './band-layout'
import type { Provider, Transport } from './provider'
import {
  referenceRecords,
  citationEntry,
  type ReferenceEntry,
  type CitationItem,
  type Citation,
} from './citation-index'
import { tr } from './i18n'
import { documentContent } from './shared-inline-marks'
import { applyObjectEdit, inlineTarget, treeTarget } from './object-commands'
import { errorText } from './command-error'

/** Citation content owns only its fields; placement, focus, Escape and actions use the shared band. */
export function citationPanel(
  editor: Editor,
  provider: Provider,
  transport: Transport,
  bands: BandHost,
) {
  let opened: { node?: PMNode; items: CitationItem[]; form: Citation['form'] } | undefined
  let ticket = 0,
    pending = false,
    results: ReferenceEntry[] = []
  const panel = contextPanel({
    id: 'citation-properties',
    closeLabel: tr('Annuler'),
    close: () => {
      if (!pending) bands.close()
    },
    canConfirm: () => !!opened?.items.length && !pending && !editor.view.composing,
    tabExit: () => focusEditorSelection(editor),
  })
  panel.title.textContent = tr('Citer une référence')
  const input = document.createElement('input')
  input.type = 'search'
  input.placeholder = tr('Rechercher par titre ou auteur…')
  input.setAttribute('aria-label', tr('Rechercher une référence'))
  input.setAttribute('role', 'combobox')
  input.setAttribute('aria-autocomplete', 'list')
  input.setAttribute('aria-expanded', 'true')
  input.dataset.initialFocus = ''
  const choices = document.createElement('div')
  choices.id = 'citation-search-results'
  choices.className = 'command-palette-results'
  choices.role = 'listbox'
  choices.setAttribute('aria-label', tr('Références'))
  input.setAttribute('aria-controls', choices.id)
  panel.recoveryData = () =>
    opened ? { new: !opened.node, items: opened.items, form: opened.form } : null
  panel.restoreRecoveryData = (data) => {
    if (!opened || !data || !Array.isArray(data.items)) return false
    opened.items = structuredClone(data.items)
    opened.form = data.form
    form.value = data.form
    renderChosen()
    return true
  }
  const chosen = document.createElement('div')
  chosen.className = 'citation-selected'
  const formLabel = document.createElement('label')
  formLabel.textContent = tr('Forme de la citation')
  const form = document.createElement('select')
  form.setAttribute('aria-label', tr('Forme de la citation'))
  form.add(new Option(tr('Standard'), 'normal'))
  form.add(new Option(tr('Dans la phrase'), 'prose'))
  form.onchange = () => {
    if (opened) opened.form = form.value as Citation['form']
  }
  formLabel.append(form)
  panel.body.append(input, choices, chosen, formLabel)
  const navigation = navigationGroup(choices, {
    axis: 'vertical',
    input,
    onActivate: (element) => {
      const result = results[Number(element.dataset.entry)]
      if (!result || !opened) return
      if (!opened.items.some((item) => item.record_id === result.record_id))
        opened.items.push({ record_id: result.record_id, supplement: '' })
      renderChosen()
      input.focus()
      void search()
    },
  })
  async function search() {
    const current = ++ticket
    // Never let a keystroke activate results belonging to the preceding query.
    results = []
    choices.replaceChildren()
    navigation.refresh(true)
    try {
      const entries = await transport.invoke<ReferenceEntry[]>('bibliography_search', {
        files: referenceRecords(provider),
        query: input.value,
      })
      if (current !== ticket || !opened) return
      results = entries
      choices.replaceChildren()
      for (const [index, entry] of entries.entries()) {
        const button = document.createElement('button')
        button.type = 'button'
        button.tabIndex = -1
        button.dataset.entry = String(index)
        const title = document.createElement('strong'),
          detail = document.createElement('span')
        title.textContent = entry.title || entry.key
        detail.textContent = `${entry.authors} · ${entry.year} · ${entry.key}`
        button.append(title, detail)
        button.disabled = opened.items.some((item) => item.record_id === entry.record_id)
        choices.append(button)
      }
      navigation.refresh(true)
      panel.error.textContent = entries.length ? '' : tr('Aucune référence ne correspond.')
      bands.reveal()
    } catch (error) {
      if (current === ticket && opened) panel.error.textContent = errorText(error)
    }
  }
  function renderChosen() {
    chosen.replaceChildren()
    for (const item of opened?.items || []) {
      const row = document.createElement('div')
      row.className = 'citation-selected-row'
      const label = document.createElement('span')
      label.textContent =
        citationEntry(editor, item.record_id)?.key ||
        results.find((entry) => entry.record_id === item.record_id)?.key ||
        '?'
      const supplement = document.createElement('input')
      supplement.placeholder = tr('Pages ou chapitre (facultatif)')
      supplement.setAttribute('aria-label', `${tr('Pages ou chapitre')} · ${label.textContent}`)
      supplement.value = item.supplement
      supplement.oninput = () => (item.supplement = supplement.value)
      const actions = actionRow(),
        remove = document.createElement('button')
      remove.type = 'button'
      remove.textContent = '×'
      remove.setAttribute('aria-label', `${tr('Retirer de la citation')} · ${label.textContent}`)
      remove.onclick = () => {
        if (opened) opened.items = opened.items.filter((candidate) => candidate !== item)
        renderChosen()
        void search()
      }
      actions.append(remove)
      row.append(label, supplement, actions)
      chosen.append(row)
    }
  }
  editor.on('transaction', ({ transaction }) => {
    if (opened && transaction.getMeta('bibliography-index')) void search()
  })
  input.oninput = () => void search()
  panel.action({
    label: tr('Créer une notice…'),
    run: () => {
      const state = opened,
        target = bands.target
      if (!state || !target || pending) return
      openBibliography(editor, provider, transport, {
        create: true,
        cite: {
          node: {
            type: 'citation',
            attrs: { items: structuredClone(state.items), form: state.form },
            marks: state.node?.marks.map((mark) => mark.toJSON()),
          },
          target: inlineTarget(editor, target.from, target.to) as unknown as Record<
            string,
            unknown
          >,
          document: documentContent(editor.getJSON()),
          position: target.from,
        },
        done: () => {
          panel.committed()
          bands.close(true, editor.state.selection.getBookmark())
        },
      })
    },
  })
  panel.action({
    label: tr('Annuler'),
    run: () => {
      if (!pending) bands.close()
    },
  })
  const apply = panel.action({ label: tr('Valider'), primary: true, run: () => panel.submit() })
  panel.confirm = () => {
    void confirm()
  }
  async function confirm() {
    const state = opened,
      target = bands.target
    if (!state || !target || !state.items.length || pending || editor.view.composing) return
    const node = editor.state.doc.nodeAt(target.from)
    if (state.node && (!node || !state.node.eq(node))) {
      panel.error.textContent = tr(
        'Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.',
      )
      return
    }
    const attrs = { items: state.items.map((item) => ({ ...item })), form: state.form }
    const replacement = editor.schema.nodes.citation.create(attrs, undefined, state.node?.marks)
    const trn = state.node
      ? editor.state.tr.setNodeMarkup(target.from, undefined, { ...state.node.attrs, ...attrs })
      : editor.state.tr.replaceWith(target.from, target.to, replacement)
    trn.setSelection(TextSelection.create(trn.doc, target.from + replacement.nodeSize))
    const commands = editingProvider(editor)
      ? [
          state.node
            ? { id: 'node.set_attrs', params: { target: treeTarget(editor, target.from), attrs } }
            : {
                id: 'inline.replace',
                params: {
                  target: inlineTarget(editor, target.from, target.to),
                  node: documentContent(replacement.toJSON()),
                },
              },
        ]
      : undefined
    pending = true
    apply.disabled = true
    panel.body.inert = true
    try {
      await applyObjectEdit(editor, trn, commands, {
        focus: false,
        current: () => opened === state,
      })
      panel.committed()
      bands.close(true, editor.state.selection.getBookmark())
    } catch (error) {
      if (opened === state) panel.error.textContent = errorText(error)
    } finally {
      pending = false
      apply.disabled = false
      panel.body.inert = false
    }
  }
  return {
    open(pos?: number) {
      if (pending || !editingAvailability(editor).enabled) return
      const node = pos === undefined ? undefined : editor.state.doc.nodeAt(pos)
      if (pos !== undefined && node?.type.name !== 'citation') return
      const selected = {
        node: node || undefined,
        items: structuredClone(node?.attrs.items || []) as CitationItem[],
        form: (node?.attrs.form || 'normal') as Citation['form'],
      }
      const from = pos ?? editor.state.selection.from,
        to = pos === undefined ? editor.state.selection.to : pos + node!.nodeSize
      bands.close(false)
      opened = selected
      input.value = ''
      form.value = selected.form
      panel.error.textContent = ''
      renderChosen()
      bands.open(panel, { from, to, placement: 'text' }, () => {
        opened = undefined
        ++ticket
      })
      opened = selected
      void search()
    },
  }
}

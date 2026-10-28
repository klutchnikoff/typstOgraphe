import { navigationGroup } from './keyboard-navigation'
import { tr } from './i18n'
import { Extension, type Editor } from '@tiptap/core'
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import type { Node as PMNode } from '@tiptap/pm/model'
import type { Transport, Provider } from './provider'
import { settingsFor } from './state'
import type { BandHost } from './band-layout'
import { contextPanel } from './context-panel'
import { replaceText } from './text-edits'
import { inlineTarget } from './object-commands'
import { executeStructured } from './structured-commands'
export interface Hit {
  from: number
  to: number
  word: string
}
interface State {
  query: string
  ignoreCase: boolean
  hits: Hit[]
  issues: Hit[]
  decorations: DecorationSet
}
export const proofingKey = new PluginKey<State>('document-proofing')
export function findMatches(doc: PMNode, query: string, ignoreCase: boolean): Hit[] {
  if (!query) return []
  const pattern = new RegExp(
      query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
      ignoreCase ? 'giu' : 'gu',
    ),
    hits: Hit[] = []
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return
    const text = node.textBetween(0, node.content.size, '\n', '\ufffc')
    for (const match of text.matchAll(pattern)) {
      // Object placeholders and returns are boundaries, never replaceable text.
      if (/[\n\ufffc]/.test(match[0])) continue
      hits.push({
        from: pos + 1 + match.index!,
        to: pos + 1 + match.index! + match[0].length,
        word: match[0],
      })
    }
    return false
  })
  return hits
}
function decorations(doc: PMNode, hits: Hit[], issues: Hit[]) {
  return DecorationSet.create(doc, [
    ...hits.map((hit) => Decoration.inline(hit.from, hit.to, { class: 'search-hit' })),
    ...issues.map((issue) =>
      Decoration.inline(issue.from, issue.to, { class: 'spelling-error', 'data-word': issue.word }),
    ),
  ])
}
export const Proofing = Extension.create({
  name: 'proofing',
  addProseMirrorPlugins() {
    return [
      new Plugin<State>({
        key: proofingKey,
        state: {
          init: (_, state) => ({
            query: '',
            ignoreCase: true,
            hits: [],
            issues: [],
            decorations: DecorationSet.empty,
          }),
          apply(tr, previous, _, next) {
            const meta = tr.getMeta(proofingKey) as Partial<State> | undefined
            const query = meta?.query ?? previous.query,
              ignoreCase = meta?.ignoreCase ?? previous.ignoreCase
            const hits =
              tr.docChanged || meta?.query !== undefined || meta?.ignoreCase !== undefined
                ? findMatches(next.doc, query, ignoreCase)
                : previous.hits
            const issues =
              meta?.issues ??
              (tr.docChanged
                ? previous.issues
                    .map((issue) => ({
                      ...issue,
                      from: tr.mapping.map(issue.from),
                      to: tr.mapping.map(issue.to),
                    }))
                    .filter(
                      (issue) =>
                        issue.from < issue.to &&
                        next.doc.textBetween(issue.from, issue.to) === issue.word,
                    )
                : previous.issues)
            return {
              query,
              ignoreCase,
              hits,
              issues,
              decorations: decorations(next.doc, hits, issues),
            }
          },
        },
        props: {
          decorations: (state) => proofingKey.getState(state)?.decorations,
          handleClick(view, pos, event) {
            if (!(event.target instanceof Element) || !event.target.closest('.search-hit'))
              return false
            const hit = proofingKey
              .getState(view.state)
              ?.hits.find((hit) => hit.from <= pos && pos <= hit.to)
            if (!hit) return false
            view.dispatch(
              view.state.tr.setSelection(TextSelection.create(view.state.doc, hit.from, hit.to)),
            )
            return true
          },
        },
      }),
    ]
  },
})
export function searchState(editor: Editor, query: string, ignoreCase = true) {
  editor.view.dispatch(editor.state.tr.setMeta(proofingKey, { query, ignoreCase }))
  return proofingKey.getState(editor.state)?.hits || []
}
export function correctionAtCursor(editor: Editor) {
  if (!(editor.state.selection instanceof TextSelection)) return
  const position = editor.state.selection.from
  return proofingKey
    .getState(editor.state)
    ?.issues.find((issue) => issue.from <= position && position <= issue.to)
}
export function navigateSpelling(editor: Editor, backwards = false) {
  const hits = proofingKey.getState(editor.state)?.issues || []
  const { from, to } = editor.state.selection
  const hit = backwards
    ? [...hits].reverse().find((h) => h.to <= from) || hits.at(-1)
    : hits.find((h) => h.from >= to && h.from !== from) || hits[0]
  if (!hit) return false
  return editor
    .chain()
    .focus()
    .setTextSelection({ from: hit.from, to: hit.to })
    .scrollIntoView()
    .run()
}
export function openCorrection(editor: Editor) {
  const issue = correctionAtCursor(editor)
  if (!issue) return false
  document.dispatchEvent(new CustomEvent('proofing:open', { detail: issue }))
  return true
}
export function mountProofing(
  editor: Editor,
  bands: BandHost,
  provider: Provider,
  transport: Transport,
  preferences: () => { personal: Record<string, string[]>; spelling_enabled: boolean },
  savePersonal: (word: string, lang: string) => Promise<void>,
  message: (s: string, error?: boolean) => void,
) {
  let timer: ReturnType<typeof setTimeout>,
    version = 0
  const check = () => {
    const ticket = ++version
    clearTimeout(timer)
    if (preferences().spelling_enabled === false) {
      editor.view.dispatch(editor.state.tr.setMeta(proofingKey, { issues: [] }))
      return
    }
    timer = setTimeout(() => {
      const segments: { text: string; pos: number }[] = []
      editor.state.doc.descendants((node, pos) => {
        if (node.isTextblock) {
          segments.push({ text: node.textBetween(0, node.content.size, '\n', ' '), pos: pos + 1 })
          return false
        }
      })
      const settings = settingsFor(provider),
        language = String(settings.metadata.fields.options.language)
      const ignored = [
        ...(preferences().personal[language] || []),
        ...((
          settings.metadata.fields.options.spelling_ignored as Record<string, string[]> | undefined
        )?.[language] || []),
      ]
      void transport
        .invoke<{ segment: number; from: number; to: number; word: string }[]>('spelling_check', {
          segments: segments.map((s) => s.text),
          language,
          ignored,
        })
        .then((found) => {
          if (ticket !== version) return
          editor.view.dispatch(
            editor.state.tr.setMeta(proofingKey, {
              issues: found.map((issue) => ({
                ...issue,
                from: segments[issue.segment].pos + issue.from,
                to: segments[issue.segment].pos + issue.to,
              })),
            }),
          )
        })
        .catch(() => {})
    }, 400)
  }
  editor.on('update', check)
  document.addEventListener('proofing:changed', check)
  editor.on('destroy', () => {
    document.removeEventListener('proofing:changed', check)
    clearTimeout(timer)
    version++
  })
  provider.doc.getMap('settings').observe(check)
  check()
  let requestTicket = 0
  const open = (event: Event) => {
    const issue = (event as CustomEvent<Hit>).detail
    if (
      editor.view.composing ||
      issue.to > editor.state.doc.content.size ||
      editor.state.doc.textBetween(issue.from, issue.to) !== issue.word
    )
      return
    const ticket = ++requestTicket
    let pending = false
    const language = String(settingsFor(provider).metadata.fields.options.language)
    const close = () => {
      if (bands.forPanel(panel.element)) bands.close()
    }
    const panel = contextPanel({
      id: 'proofing-band',
      closeLabel: tr('Fermer la correction'),
      close,
      canConfirm: () => !pending && !editor.view.composing,
      tabExit: () => editor.view.focus(),
    })
    const band = panel.element
    const kind = document.createElement('span')
    kind.className = 'band-kind'
    kind.textContent = tr('Orthographe')
    panel.title.append(kind, document.createTextNode(`« ${issue.word} »`))
    const choices = document.createElement('div')
    choices.className = 'suggestions'
    choices.dataset.defaultNavigation = ''
    navigationGroup(choices, { axis: 'both', roving: true })
    choices.textContent = tr('Recherche de suggestions…')
    panel.body.append(choices)
    const target = () => bands.forPanel(band)
    const valid = () => {
      const range = target()
      if (!range) return null
      if (range.from > range.to || range.to > editor.state.doc.content.size) return null
      if (
        String(settingsFor(provider).metadata.fields.options.language) !== language ||
        editor.state.doc.textBetween(range.from, range.to) !== issue.word
      ) {
        panel.error.textContent = tr('Le mot a changé : rouvrez la correction.')
        return null
      }
      return range
    }
    const commit = async (change: (current: () => boolean) => Promise<unknown>) => {
      if (pending || !valid()) return
      const current = () => ticket === requestTicket && target() !== null
      pending = true
      panel.error.textContent = ''
      const controls = [
        ...band.querySelectorAll<HTMLButtonElement>('button:not(.panel-title button)'),
      ]
      for (const button of controls) button.disabled = true
      try {
        await change(current)
        if (current()) bands.close(true, editor.state.selection.getBookmark())
      } catch (reason) {
        if (current())
          panel.error.textContent = reason instanceof Error ? reason.message : String(reason)
      } finally {
        pending = false
        for (const button of controls) button.disabled = false
      }
    }
    panel.action({
      label: tr('Ignorer dans ce document'),
      run: () => {
        const range = valid()
        if (!range) return
        void commit((current) =>
          executeStructured(
            editor,
            provider,
            transport,
            [
              {
                id: 'spelling.ignore',
                params: {
                  target: inlineTarget(editor, range.from, range.to),
                  language,
                  word: issue.word,
                },
              },
            ],
            current,
          ),
        )
      },
    })
    panel.action({
      label: tr('Ajouter au dictionnaire personnel'),
      run: () => {
        if (!valid()) return
        void savePersonal(issue.word, language)
          .then(() => {
            preferences().personal[language] = [
              ...(preferences().personal[language] || []),
              issue.word,
            ]
            close()
            check()
          })
          .catch((reason) => message(String(reason), true))
      },
    })
    bands.open(panel, { from: issue.from, to: issue.to, placement: 'text' }, () => band.remove())
    void transport
      .invoke<string[]>('spelling_suggestions', { language, word: issue.word })
      .then((suggestions) => {
        if (ticket !== requestTicket || !target()) return
        choices.replaceChildren()
        const buttons = suggestions.slice(0, 6).map((suggestion) => {
          return panel.action({
            label: suggestion,
            parent: choices,
            run: () => {
              const range = valid()
              if (!range) return
              void commit((current) =>
                replaceText(editor, [{ ...range, word: issue.word, text: suggestion }], {
                  focus: false,
                  current,
                }),
              )
            },
          })
        })
        const [preferred, ...others] = buttons
        if (preferred) {
          panel.setDefault(preferred)
          choices.append(...others, preferred)
        } else choices.textContent = tr('Aucune suggestion disponible.')
      })
      .catch((reason) => {
        if (target()) panel.error.textContent = String(reason)
      })
  }
  document.addEventListener('proofing:open', open)
  editor.on('destroy', () => {
    clearTimeout(timer)
    version++
    requestTicket++
    editor.off('update', check)
    provider.doc.getMap('settings').unobserve(check)
    document.removeEventListener('proofing:open', open)
  })
}

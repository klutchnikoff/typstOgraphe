import { tr } from './i18n'
import { Extension, type Editor } from '@tiptap/core'
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import type { Node as PMNode } from '@tiptap/pm/model'
import type { Transport, Provider } from './provider'
import { settingsFor, setSetting } from './state'
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
    for (const match of text.matchAll(pattern))
      hits.push({
        from: pos + 1 + match.index!,
        to: pos + 1 + match.index! + match[0].length,
        word: match[0],
      })
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
export function openCorrection(editor: Editor) {
  const position = editor.state.selection.from
  const issue = proofingKey
    .getState(editor.state)
    ?.issues.find((issue) => issue.from <= position && position <= issue.to)
  if (!issue) return false
  document.dispatchEvent(new CustomEvent('proofing:open', { detail: issue }))
  return true
}
export function mountProofing(
  editor: Editor,
  provider: Provider,
  transport: Transport,
  personal: Record<string, string[]>,
  savePersonal: (word: string, lang: string) => Promise<void>,
  message: (s: string, error?: boolean) => void,
) {
  let timer: ReturnType<typeof setTimeout>,
    version = 0
  const check = () => {
    const ticket = ++version
    clearTimeout(timer)
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
        ...(personal[language] || []),
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
  provider.doc.getMap('settings').observe(check)
  check()
  document.addEventListener('proofing:open', (event) => {
    const issue = (event as CustomEvent<Hit>).detail
    void (async () => {
      if (issue.from < 0 || issue.to > editor.state.doc.content.size) return
      const original = editor.state.doc.textBetween(issue.from, issue.to)
      if (original !== issue.word) return
      const settings = settingsFor(provider),
        language = String(settings.metadata.fields.options.language)
      const suggestions = await transport.invoke<string[]>('spelling_suggestions', {
        language,
        word: issue.word,
      })
      document.querySelector<HTMLElement>('#properties')!.hidden = true
      document.querySelector('#proofing-band')?.remove()
      const band = document.createElement('section')
      band.id = 'proofing-band'
      band.className = 'properties'
      const title = document.createElement('strong')
      title.textContent = `Orthographe · ${issue.word}`
      const actions = document.createElement('div')
      actions.className = 'panel-actions'
      const close = () => {
        band.remove()
        editor.commands.focus()
      }
      for (const suggestion of suggestions.slice(0, 6)) {
        const button = document.createElement('button')
        button.textContent = suggestion
        button.onclick = () => {
          if (
            issue.to > editor.state.doc.content.size ||
            editor.state.doc.textBetween(issue.from, issue.to) !== issue.word
          ) {
            message(tr('Le mot a changé : rouvrez la correction.'), true)
            close()
            return
          }
          editor
            .chain()
            .focus()
            .insertContentAt({ from: issue.from, to: issue.to }, suggestion)
            .run()
          close()
        }
        actions.append(button)
      }
      const ignore = document.createElement('button')
      ignore.textContent = tr('Ignorer dans ce document')
      ignore.onclick = () => {
        const ignored = structuredClone(
          (settingsFor(provider).metadata.fields.options.spelling_ignored || {}) as Record<
            string,
            string[]
          >,
        )
        ignored[language] = [...new Set([...(ignored[language] || []), issue.word])]
        setSetting(provider, 'options.spelling_ignored', ignored)
        close()
        check()
      }
      const add = document.createElement('button')
      add.textContent = tr('Ajouter au dictionnaire personnel')
      add.onclick = () => {
        void savePersonal(issue.word, language)
          .then(() => {
            personal[language] = [...(personal[language] || []), issue.word]
            close()
            check()
          })
          .catch((error) => message(String(error), true))
      }
      const cancel = document.createElement('button')
      cancel.textContent = tr('Fermer')
      cancel.onclick = close
      actions.append(ignore, add, cancel)
      band.append(title, actions)
      document.querySelector('#editor-scroll')!.before(band)
      band.tabIndex = -1
      band.focus()
      band.onkeydown = (e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          close()
        }
        if (e.key === 'Enter') {
          e.preventDefault()
          actions.querySelector<HTMLButtonElement>('button')?.click()
        }
      }
    })().catch((error) => message(String(error), true))
  })
}

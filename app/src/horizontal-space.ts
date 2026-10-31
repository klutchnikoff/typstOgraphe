import { Extension } from '@tiptap/core'
import { Plugin, PluginKey, TextSelection, type Transaction } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import * as Y from 'yjs'
import type { Node as PMNode } from '@tiptap/pm/model'

export const nonbreakingSpace = '\u00a0'
const space = (value: string) => value === ' ' || value === nonbreakingSpace
/** Return UTF-16 offsets to delete, keeping the first space of each run. */
function duplicates(text: string, previous: boolean) {
  const remove: number[] = []
  for (let i = 0; i < text.length; i++) {
    const current = space(text[i])
    if (current && previous) remove.push(i)
    previous = current
  }
  return { remove, previous }
}
export function normalizeSpaces(tr: Transaction) {
  const remove: number[] = []
  tr.doc.descendants((parent, pos) => {
    if (!parent.isTextblock) return
    let previous = false
    parent.forEach((node, offset) => {
      const result = duplicates(node.isText ? node.text! : '\ufffc', previous)
      remove.push(...result.remove.map((index) => pos + 1 + offset + index))
      previous = result.previous
    })
    return false
  })
  for (const pos of remove.reverse()) tr.delete(pos, pos + 1)
  return tr
}
/** Delete individual CRDT characters, preserving surrounding text identities,
 * styles and selections, including across adjacent marked text objects. */
export function normalizeSharedSpaces(parent: Y.XmlFragment) {
  let previous = false
  for (const child of parent.toArray()) {
    if (child instanceof Y.XmlText) {
      const plain = child
        .toDelta()
        .map((part: { insert: unknown }) =>
          typeof part.insert === 'string' ? part.insert : '\ufffc',
        )
        .join('')
      const result = duplicates(plain, previous)
      previous = result.previous
      for (const index of result.remove.reverse()) child.delete(index, 1)
    } else {
      previous = false
      if (child instanceof Y.XmlElement) normalizeSharedSpaces(child)
    }
  }
}

const spaceKey = new PluginKey<DecorationSet>('nonbreaking-spaces')
function spaceDecorations(doc: PMNode) {
  const decorations: Decoration[] = []
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text?.includes(nonbreakingSpace)) return
    for (let i = 0; i < node.text.length; i++)
      if (node.text[i] === nonbreakingSpace)
        decorations.push(Decoration.inline(pos + i, pos + i + 1, { class: 'nonbreaking-space' }))
  })
  return DecorationSet.create(doc, decorations)
}

/** One input policy and one visual projection for every rich-zone profile. */
export const HorizontalSpace = Extension.create({
  name: 'horizontalSpace',
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: spaceKey,
        state: {
          init: (_, state) => spaceDecorations(state.doc),
          apply: (tr, previous) => (tr.docChanged ? spaceDecorations(tr.doc) : previous),
        },
        props: {
          handleTextInput(view, from, to, text) {
            if (view.composing || !space(text)) return false
            const $from = view.state.doc.resolve(from),
              $to = view.state.doc.resolve(to)
            if (!$from.sameParent($to) || !$from.parent.isTextblock) return false
            const before = $from.parent.textBetween(0, $from.parentOffset, '', '\ufffc')
            const after = $to.parent.textBetween(
              $to.parentOffset,
              $to.parent.content.size,
              '',
              '\ufffc',
            )
            const left = before.match(/[ \u00a0]+$/)?.[0] || ''
            const right = after.match(/^[ \u00a0]+/)?.[0] || ''
            const existing = left || right
            // Explicit NBSP input is preserved. Only repeated ordinary Space cycles.
            const value =
              from === to && text === ' ' && existing
                ? existing[0] === ' '
                  ? nonbreakingSpace
                  : ' '
                : text
            const tr = view.state.tr.insertText(value, from - left.length, to + right.length)
            tr.setSelection(TextSelection.create(tr.doc, from - left.length + 1))
            if (view.state.storedMarks !== null) tr.setStoredMarks(view.state.storedMarks)
            view.dispatch(tr.scrollIntoView())
            return true
          },
          decorations(state) {
            return spaceKey.getState(state) || null
          },
        },
      }),
    ]
  },
})

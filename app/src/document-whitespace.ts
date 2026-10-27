import * as Y from 'yjs'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Extension } from '@tiptap/core'
import { ySyncPluginKey } from '@tiptap/y-tiptap'
import { Plugin, PluginKey, type Transaction } from '@tiptap/pm/state'

export const documentWhitespaceKey = new PluginKey('documentWhitespace')
export const lineBreak = (node: PMNode | null | undefined) =>
  !!node && ['softBreak', 'hardBreak'].includes(node.type.name)
const emptyParagraph = (node: PMNode | null | undefined) =>
  node?.type.name === 'paragraph' && node.content.content.every((child) => lineBreak(child))
const emptyFlowBlock = (node: PMNode | null | undefined) =>
  !!node &&
  ['paragraph', 'heading'].includes(node.type.name) &&
  node.content.content.every((child) => lineBreak(child))

/** Repair whitespace without rewriting text, marks, annotations, list items or cells. */
export function normalizeWhitespace(tr: Transaction, trailingParagraph = true) {
  // Keep an empty document editable, including a heading chosen before typing.
  if (tr.doc.content.content.some((node) => !emptyFlowBlock(node)))
    while (emptyFlowBlock(tr.doc.firstChild)) tr.delete(0, tr.doc.firstChild!.nodeSize)
  const first = tr.doc.firstChild
  if (first && ['paragraph', 'heading'].includes(first.type.name)) {
    let leading = 0
    for (const child of first.content.content) {
      if (!lineBreak(child)) break
      leading += child.nodeSize
    }
    if (leading) tr.delete(1, 1 + leading)
  }
  const remove: { from: number; to: number }[] = []
  const visit = (parent: PMNode, start: number) => {
    parent.forEach((node, offset, index) => {
      const pos = start + offset
      if (
        ['doc', 'quote'].includes(parent.type.name) &&
        emptyParagraph(node) &&
        emptyParagraph(parent.maybeChild(index + 1))
      ) {
        remove.push({ from: pos, to: pos + node.nodeSize })
        return
      }
      if (node.type.name === 'paragraph') {
        const empty = emptyParagraph(node)
        let previousBreak = false
        node.forEach((child, inner) => {
          const currentBreak = lineBreak(child)
          if (currentBreak && (empty || previousBreak))
            remove.push({ from: pos + 1 + inner, to: pos + 1 + inner + child.nodeSize })
          previousBreak = currentBreak
        })
      } else if (node.childCount) visit(node, pos + 1)
    })
  }
  visit(tr.doc, 0)
  for (const range of remove.sort((a, b) => b.from - a.from)) tr.delete(range.from, range.to)

  // A final inline break already supplies the empty editing line. Do not add a second one.
  const last = tr.doc.lastChild
  const previous = tr.doc.maybeChild(tr.doc.childCount - 2)
  if (emptyParagraph(last) && previous?.type.name === 'paragraph' && lineBreak(previous.lastChild))
    tr.delete(tr.doc.content.size - last!.nodeSize, tr.doc.content.size)
  const end = tr.doc.lastChild
  if (
    trailingParagraph &&
    (end?.type.name !== 'paragraph' || (end.content.size > 0 && !lineBreak(end.lastChild)))
  )
    tr.insert(tr.doc.content.size, tr.doc.type.schema.nodes.paragraph.create())
  return tr
}

/** Shared repairs delete only the Yjs whitespace items, preserving text identities for undo. */
function normalizeSharedWhitespace(
  root: Y.XmlFragment,
  origin: unknown = documentWhitespaceKey,
  addToHistory = false,
  trailingParagraph = true,
) {
  const isBreak = (node: unknown) =>
    node instanceof Y.XmlElement && ['softBreak', 'hardBreak'].includes(node.nodeName)
  const empty = (node: unknown): node is Y.XmlElement =>
    node instanceof Y.XmlElement &&
    node.nodeName === 'paragraph' &&
    node
      .toArray()
      .every((child) => (child instanceof Y.XmlText ? child.length === 0 : isBreak(child)))
  const emptyFlow = (node: unknown): node is Y.XmlElement =>
    node instanceof Y.XmlElement &&
    ['paragraph', 'heading'].includes(node.nodeName) &&
    node
      .toArray()
      .every((child) => (child instanceof Y.XmlText ? child.length === 0 : isBreak(child)))
  const visit = (parent: Y.XmlFragment, flow: boolean) => {
    const children = parent.toArray()
    for (let i = children.length - 1; i >= 0; i--) {
      const node = children[i]
      if (!(node instanceof Y.XmlElement)) continue
      if (flow && empty(node) && empty(children[i + 1])) {
        parent.delete(i, 1)
        continue
      }
      if (node.nodeName === 'paragraph') {
        const parts = node.toArray()
        let previous = false
        const indices: number[] = []
        parts.forEach((part, index) => {
          if (part instanceof Y.XmlText && part.length === 0) return
          const current = isBreak(part)
          if (current && (empty(node) || previous)) indices.push(index)
          previous = current
        })
        indices.reverse().forEach((index) => node.delete(index, 1))
      } else visit(node, node.nodeName === 'quote')
    }
  }
  root.doc!.transact((transaction) => {
    transaction.meta.set('addToHistory', addToHistory)
    if (root.toArray().some((node) => !emptyFlow(node)))
      while (emptyFlow(root.toArray()[0])) root.delete(0, 1)
    const first = root.toArray()[0]
    if (first instanceof Y.XmlElement && ['paragraph', 'heading'].includes(first.nodeName)) {
      const leading: number[] = []
      for (const [index, child] of first.toArray().entries()) {
        if (child instanceof Y.XmlText && child.length === 0) continue
        if (!isBreak(child)) break
        leading.push(index)
      }
      leading.reverse().forEach((index) => first.delete(index, 1))
    }
    visit(root, true)
    const children = root.toArray(),
      last = children.at(-1),
      previous = children.at(-2)
    const endingBreak = (node: unknown) =>
      node instanceof Y.XmlElement &&
      node.nodeName === 'paragraph' &&
      isBreak(
        node
          .toArray()
          .filter((part) => !(part instanceof Y.XmlText) || part.length > 0)
          .at(-1),
      )
    if (empty(last) && endingBreak(previous)) root.delete(root.length - 1, 1)
    const end = root.toArray().at(-1)
    if (trailingParagraph && !empty(end) && !endingBreak(end))
      root.insert(root.length, [new Y.XmlElement('paragraph')])
  }, origin)
}

/** Keep local repairs in the same undo capture; preserve shared character identities on remote repairs. */
export const DocumentWhitespace = Extension.create({
  name: 'documentWhitespace',
  addOptions: () => ({ trailingParagraph: true }),
  addStorage: () => ({
    checked: false,
    repairing: false,
    timer: undefined as ReturnType<typeof setTimeout> | undefined,
  }),
  dispatchTransaction({ transaction, next }) {
    if (this.editor.isCapturingTransaction || this.storage.repairing) return next(transaction)
    const check =
      !this.storage.checked || transaction.docChanged || transaction.getMeta(documentWhitespaceKey)
    this.storage.checked = true
    if (transaction.getMeta(ySyncPluginKey)?.isChangeOrigin) {
      // Remote/undo transactions must first install their shared projection unchanged.
      next(transaction)
      clearTimeout(this.storage.timer)
      this.storage.timer = setTimeout(() => {
        this.storage.timer = undefined
        if (!this.editor.isDestroyed && !this.editor.view.composing) {
          const shared = ySyncPluginKey.getState(this.editor.state)
          if (shared)
            normalizeSharedWhitespace(
              shared.type,
              documentWhitespaceKey,
              false,
              this.options.trailingParagraph,
            )
        }
      }, 0)
      return
    }
    next(transaction)
    if (check && !this.editor.view.composing) {
      const shared = ySyncPluginKey.getState(this.editor.state)
      this.storage.repairing = true
      try {
        if (shared)
          normalizeSharedWhitespace(
            shared.type,
            ySyncPluginKey,
            transaction.getMeta('addToHistory') !== false,
            this.options.trailingParagraph,
          )
        else {
          const repair = normalizeWhitespace(this.editor.state.tr, this.options.trailingParagraph)
          if (transaction.getMeta('addToHistory') === false) repair.setMeta('addToHistory', false)
          if (repair.docChanged) next(repair)
        }
      } finally {
        this.storage.repairing = false
      }
    }
  },
  onDestroy() {
    clearTimeout(this.storage.timer)
  },
  onCreate() {
    this.editor.view.dispatch(
      this.editor.state.tr.setMeta(documentWhitespaceKey, true).setMeta('addToHistory', false),
    )
  },
  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleDOMEvents: {
            compositionend: (view) => {
              queueMicrotask(() => {
                if (!view.isDestroyed)
                  view.dispatch(view.state.tr.setMeta(documentWhitespaceKey, true))
              })
              return false
            },
          },
        },
      }),
    ]
  },
})

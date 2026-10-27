import { Extension } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { isInlineObject } from './inline-objects'

export const inlineObjectNumberingKey = new PluginKey<DecorationSet>('inline-object-numbering')

function numberedObjects(doc: PMNode, kinds: Set<string>) {
  const counts = new Map<string, number>(),
    decorations: Decoration[] = []
  doc.descendants((node, pos) => {
    if (!kinds.has(node.type.name) || !isInlineObject(node)) return
    const number = (counts.get(node.type.name) || 0) + 1
    counts.set(node.type.name, number)
    decorations.push(
      Decoration.node(
        pos,
        pos + node.nodeSize,
        { 'data-inline-number': String(number) },
        { inlineObjectNumber: number },
      ),
    )
    return false
  })
  return DecorationSet.create(doc, decorations)
}

/** Local presentation, derived once per document change; never shared or saved. */
export const InlineObjectNumbering = Extension.create<{ kinds: string[] }>({
  name: 'inlineObjectNumbering',
  addOptions: () => ({ kinds: [] }),
  addProseMirrorPlugins() {
    const kinds = new Set(this.options.kinds)
    return [
      new Plugin<DecorationSet>({
        key: inlineObjectNumberingKey,
        state: {
          init: (_, state) => numberedObjects(state.doc, kinds),
          apply: (transaction, previous) =>
            transaction.docChanged ? numberedObjects(transaction.doc, kinds) : previous,
        },
        props: { decorations: (state) => inlineObjectNumberingKey.getState(state) },
      }),
    ]
  },
})

export function inlineObjectNumber(decorations: readonly Decoration[]) {
  return decorations.find((decoration) => typeof decoration.spec.inlineObjectNumber === 'number')
    ?.spec.inlineObjectNumber as number | undefined
}

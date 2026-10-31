import { Extension } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { isInlineObject } from './inline-objects'

const key = new PluginKey<DecorationSet>('inline-object-spacing')
function spacing(doc: PMNode) {
  const decorations: Decoration[] = []
  doc.descendants((parent, pos) => {
    if (!parent.isTextblock) return
    let previous: PMNode | undefined
    parent.forEach((node, offset) => {
      if (
        isInlineObject(node) &&
        (!previous ||
          ['softBreak', 'hardBreak'].includes(previous.type.name) ||
          (previous.isText && /[ \u00a0\u202f\n]$/u.test(previous.text!)))
      ) {
        decorations.push(
          Decoration.node(pos + 1 + offset, pos + 1 + offset + node.nodeSize, {
            class: 'inline-object-flush',
          }),
        )
      }
      previous = node
    })
    return false
  })
  return DecorationSet.create(doc, decorations)
}

/** Display-only spacing, shared by every inline object. No characters, marks or
 * attributes are added to the document. Selection changes do not rescan it. */
export const InlineObjectSpacing = Extension.create({
  name: 'inlineObjectSpacing',
  addProseMirrorPlugins: () => [
    new Plugin<DecorationSet>({
      key,
      state: {
        init: (_, state) => spacing(state.doc),
        apply: (transaction, previous) =>
          transaction.docChanged ? spacing(transaction.doc) : previous,
      },
      props: { decorations: (state) => key.getState(state) },
    }),
  ],
})

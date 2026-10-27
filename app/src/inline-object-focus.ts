import { Extension } from '@tiptap/core'
import { Plugin, TextSelection, type EditorState } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import { isInlineObject } from './inline-objects'

/** The badge immediately before a collapsed text caret is its only active target. */
export function activeInlineObject(state: EditorState) {
  const { selection } = state
  if (!(selection instanceof TextSelection) || !selection.empty) return
  const node = selection.$from.nodeBefore
  if (isInlineObject(node)) return { pos: selection.from - node.nodeSize, node }
}

/** Presentation and contextual commands share activeInlineObject's exact boundary. */
export const InlineObjectFocus = Extension.create({
  name: 'inlineObjectFocus',
  addProseMirrorPlugins: () => [
    new Plugin({
      props: {
        decorations(state) {
          const target = activeInlineObject(state)
          return target
            ? DecorationSet.create(state.doc, [
                Decoration.node(target.pos, target.pos + target.node.nodeSize, {
                  class: 'inline-object-active',
                }),
              ])
            : DecorationSet.empty
        },
      },
    }),
  ],
})

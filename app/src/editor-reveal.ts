import { Extension, type Editor } from '@tiptap/core'
import { Plugin, PluginKey, TextSelection, type EditorState } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'

export const locationDuration = 1600
const locationKey = new PluginKey<DecorationSet>('editor-location')
const words = new Intl.Segmenter(undefined, { granularity: 'word' })
const graphemes = new Intl.Segmenter(undefined, { granularity: 'grapheme' })

/** Decoration only: never select a word just to indicate where navigation led. */
function location(state: EditorState, pos: number) {
  const $pos = state.doc.resolve(pos),
    parent = $pos.parent
  if (!parent.isTextblock || !$pos.depth) return DecorationSet.empty
  const atom = $pos.nodeAfter
  if (atom?.isInline && !atom.isText)
    return DecorationSet.create(state.doc, [
      Decoration.node(pos, pos + atom.nodeSize, { class: 'sync-location' }),
    ])
  const text = parent.textBetween(0, parent.content.size, '', '\ufffc')
  const at = $pos.parentOffset
  const segments = words.segment(text)
  let found = segments.containing(at)
  if (!found?.isWordLike && at > 0) {
    const previous = segments.containing(at - 1)
    if (previous?.isWordLike) found = previous
  }
  const segment = found?.isWordLike ? found : graphemes.segment(text).containing(at)
  if (segment && segment.segment !== '\ufffc' && segment.segment.trim()) {
    const from = $pos.start() + segment.index
    return DecorationSet.create(state.doc, [
      Decoration.inline(from, from + segment.segment.length, { class: 'sync-location' }),
    ])
  }
  return DecorationSet.create(state.doc, [
    Decoration.node($pos.before(), $pos.after(), { class: 'sync-location' }),
  ])
}

export const EditorLocation = Extension.create({
  name: 'editorLocation',
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: locationKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, previous, _, current) {
            const request = tr.getMeta(locationKey) as { pos: number } | null | undefined
            if (request !== undefined)
              return request ? location(current, request.pos) : DecorationSet.empty
            if (tr.docChanged || tr.selectionSet || tr.storedMarksSet) return DecorationSet.empty
            return previous
          },
        },
        props: {
          decorations: (state) => locationKey.getState(state),
          handleDOMEvents: {
            pointerdown(view) {
              if (locationKey.getState(view.state) !== DecorationSet.empty)
                view.dispatch(
                  view.state.tr.setMeta(locationKey, null).setMeta('addToHistory', false),
                )
              return false
            },
          },
        },
        view(view) {
          let timer: ReturnType<typeof setTimeout> | undefined
          return {
            update(current, previous) {
              if (locationKey.getState(current.state) === locationKey.getState(previous)) return
              clearTimeout(timer)
              if (locationKey.getState(current.state) === DecorationSet.empty) return
              timer = setTimeout(() => {
                if (!current.isDestroyed)
                  current.dispatch(
                    current.state.tr.setMeta(locationKey, null).setMeta('addToHistory', false),
                  )
              }, locationDuration)
            },
            destroy: () => clearTimeout(timer),
          }
        },
      }),
    ]
  },
})

/** Center the caret in its own scroller; natural scroll limits handle the
 * beginning/end of a short document. Focus must not schedule a later scroll. */
export function revealEditorPosition(editor: Editor, pos: number) {
  if (
    editor.isDestroyed ||
    editor.view.composing ||
    !Number.isInteger(pos) ||
    pos < 0 ||
    pos > editor.state.doc.content.size
  )
    return false
  const selection = TextSelection.near(editor.state.doc.resolve(pos))
  editor.view.dispatch(
    editor.state.tr
      .setSelection(selection)
      .setMeta(locationKey, { pos: selection.head })
      .setMeta('addToHistory', false),
  )
  editor.view.focus()
  const viewport = editor.view.dom.closest<HTMLElement>('#editor-scroll')
  if (viewport) {
    const caret = editor.view.coordsAtPos(selection.head),
      bounds = viewport.getBoundingClientRect()
    const center = bounds.top + viewport.clientTop + viewport.clientHeight / 2
    viewport.scrollTo({
      top: viewport.scrollTop + (caret.top + caret.bottom) / 2 - center,
      behavior: 'instant',
    })
  }
  return true
}

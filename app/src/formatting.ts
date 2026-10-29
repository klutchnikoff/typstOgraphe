import { getMarkRange, type Editor } from '@tiptap/core'
import { Selection } from '@tiptap/pm/state'
import { capabilityAvailability } from './command-availability'
import { editingProvider } from './editing-history'
import { editorRoot, selectedTextblocks, textTargets } from './editor-targets'
import { executeStructured, type StructuredCommand } from './structured-commands'

export type Formatting =
  | 'text.bold'
  | 'text.italic'
  | 'text.clear'
  | 'text.remove_link'
  | 'text.link'
  | 'block.paragraph'
  | 'block.heading_1'
  | 'block.heading_2'
  | 'block.heading_3'
type Range = { from: number; to: number }

/** One translator for toolbar, palette, shortcuts and rich-field controls. */
export function formattingBatch(
  editor: Editor,
  id: Formatting,
  range: Range,
  url?: string,
): StructuredCommand[] {
  if (id.startsWith('text.')) {
    const value =
      id === 'text.bold' || id === 'text.italic'
        ? editor.isActive(id.slice(5))
          ? 'off'
          : 'on'
        : undefined
    return textTargets(editor, range).map((target) => ({
      id,
      params: { target, ...(value ? { value } : {}), ...(url !== undefined ? { url } : {}) },
    }))
  }
  const root = editorRoot(editor)
  if (!root) throw new Error('Missing structured editor binding.')
  const heading = /block.heading_(\d)/.exec(id)
  const command =
    heading && editor.isActive('heading', { level: Number(heading[1]) }) ? 'block.paragraph' : id
  return selectedTextblocks(editor, range)
    .filter(({ node }) => ['paragraph', 'heading'].includes(node.type.name))
    .map(({ path }) => ({ id: command, params: { target: { root, path } } }))
}

/** Collapsed styles affect future typing only. Detached record drafts stay local;
 * every explicit range edit on a bound native document is validated by its owner.
 */
export async function applyFormatting(
  editor: Editor,
  id: Formatting,
  options: { range?: Range; url?: string; focus?: boolean; current?: () => boolean } = {},
): Promise<boolean> {
  const policy = id.startsWith('block.')
    ? 'heading'
    : id === 'text.link' || id === 'text.remove_link'
      ? 'link'
      : 'style'
  const capability =
    id === 'text.remove_link' ? 'text.link' : id === 'text.clear' ? 'text.bold' : id
  if (!capabilityAvailability(editor, capability, policy).enabled) return false
  const current = options.range || editor.state.selection
  const range =
    id === 'text.remove_link' && current.from === current.to
      ? getMarkRange(editor.state.doc.resolve(current.from), editor.schema.marks.link) || current
      : current
  const selection = editor.state.selection
  const restore = () => {
    if (!editor.isDestroyed)
      editor.view.dispatch(
        editor.state.tr.setSelection(Selection.fromJSON(editor.state.doc, selection.toJSON())),
      )
  }
  const provider = editingProvider(editor)
  if (provider && (range.from !== range.to || id.startsWith('block.'))) {
    const focused = document.activeElement
    const batch: StructuredCommand[] = formattingBatch(editor, id, range, options.url)
    if (!batch.length) return false
    const changed = await executeStructured(
      editor,
      provider,
      provider.transport,
      batch,
      // Closing a menu or disabling a submit button can legitimately yield
      // focus to the body/editor. Entering another field cancels preparation.
      () =>
        (!options.current || options.current()) &&
        (document.activeElement === focused ||
          document.activeElement === document.body ||
          document.activeElement === editor.view.dom),
      restore,
    )
    // Styles and paragraph/heading conversion preserve projected positions. Yjs
    // may replace a block's XML identity; retain the user's exact selection.
    if (!editor.isDestroyed) {
      if (
        options.focus !== false &&
        (document.activeElement === focused ||
          document.activeElement === document.body ||
          editor.isFocused)
      )
        editor.view.focus()
    }
    return changed
  }
  let chain = editor.chain()
  if (options.focus !== false) chain = chain.focus()
  if (options.range) chain = chain.setTextSelection(range)
  if (id === 'text.bold') return chain.toggleBold().run()
  if (id === 'text.italic') return chain.toggleItalic().run()
  if (id === 'text.clear')
    return range.from === range.to
      ? chain.command(({ tr }) => (tr.setStoredMarks([]), true)).run()
      : chain.unsetAllMarks().run()
  if (id === 'text.link' || id === 'text.remove_link') {
    const changed = (
      id === 'text.link' ? chain.setLink({ href: options.url! }) : chain.unsetLink()
    ).run()
    if (options.range) restore()
    return changed
  }
  if (id === 'block.paragraph') return chain.setParagraph().run()
  return chain.toggleHeading({ level: Number(id.at(-1)) as 1 | 2 | 3 }).run()
}
export function runFormatting(editor: Editor, id: Formatting) {
  void applyFormatting(editor, id).catch((error: unknown) => {
    const provider = editingProvider(editor)
    if (provider) provider.onError(error)
    else throw error
  })
}

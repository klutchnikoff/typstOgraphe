import type { Editor } from '@tiptap/core'
import { tr } from './i18n'
import { inlineTarget } from './object-commands'
import { editorRoot } from './editor-targets'
import { normalizeWhitespace } from './document-whitespace'
import { applyPlannedEdit, type StructuredCommand } from './structured-commands'

export interface TextReplacement {
  from: number
  to: number
  word: string
  text: string
}

/** Search and correction edit text only: never consume a badge, a return or a zone boundary.
 * Descending ranges keep every target valid inside the single atomic batch.
 */
export function replacementPlan(editor: Editor, replacements: TextReplacement[]) {
  const transaction = editor.state.tr
  const commands: StructuredCommand[] = []
  let boundary = editor.state.doc.content.size
  for (const change of [...replacements].sort((a, b) => b.from - a.from)) {
    const { from, to, word, text } = change
    const a = editor.state.doc.resolve(from),
      b = editor.state.doc.resolve(to)
    const splitSurrogate = (offset: number) => {
      const value = a.parent.textBetween(0, a.parent.content.size, '\n', '\ufffc')
      return (
        /[\uD800-\uDBFF]/.test(value[offset - 1] || '') &&
        /[\uDC00-\uDFFF]/.test(value[offset] || '')
      )
    }
    let textOnly = true
    editor.state.doc.nodesBetween(from, to, (node) => {
      if (node.isInline && !node.isText) textOnly = false
    })
    if (
      from >= to ||
      to > boundary ||
      !a.sameParent(b) ||
      !a.parent.isTextblock ||
      !textOnly ||
      splitSurrogate(a.parentOffset) ||
      splitSurrogate(b.parentOffset) ||
      editor.state.doc.textBetween(from, to) !== word ||
      /[\r\n\ufffc]/.test(text)
    )
      throw new Error(tr('Le document a changé. Réessayez.'))
    boundary = from
    if (word === text) continue
    const marks = (a.marksAcross(b) || []).map((mark) =>
      mark.type.name === 'link' ? { type: 'link', url: mark.attrs.href } : { type: mark.type.name },
    )
    if (editorRoot(editor)) {
      const target = inlineTarget(editor, from, to)
      commands.push(
        text
          ? { id: 'text.insert', params: { target, text, marks } }
          : { id: 'text.delete', params: { target } },
      )
    }
    transaction.insertText(text, from, to)
  }
  // The owner repairs whitespace after the entire batch. Map the caret through
  // the same repair in the planned projection, using this zone's actual profile.
  const whitespace = editor.extensionManager.extensions.find(
    (extension) => extension.name === 'documentWhitespace',
  )
  if (whitespace && transaction.docChanged)
    normalizeWhitespace(transaction, whitespace.options.trailingParagraph)
  return { transaction, commands }
}

export async function replaceText(
  editor: Editor,
  replacements: TextReplacement[],
  options: Parameters<typeof applyPlannedEdit>[3] = {},
) {
  if (!editor.isEditable || editor.view.composing || editor.isDestroyed) return false
  const plan = replacementPlan(editor, replacements)
  if (!plan.transaction.docChanged) return false
  return applyPlannedEdit(editor, plan.transaction, plan.commands, options)
}

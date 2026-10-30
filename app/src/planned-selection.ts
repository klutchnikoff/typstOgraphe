import { Selection, type Transaction } from '@tiptap/pm/state'
import { ReplaceStep } from '@tiptap/pm/transform'
import { isWidgetKind } from './widget-frame'

/** A whole-widget replacement normally maps its old caret after the new node.
 * Keep an explicit destination supplied by the caller. Otherwise retain the
 * original range in identical content, or enter the replacement's first zone.
 * Deletions and partial replacements never borrow a neighboring widget.
 */
export function retainWidgetSelection(transaction: Transaction, original: Selection) {
  const step = transaction.steps[0]
  if (!(step instanceof ReplaceStep) || step.slice.content.childCount !== 1) return
  const before = transaction.before.nodeAt(step.from)
  const inserted = step.slice.content.firstChild!
  if (
    !before ||
    !isWidgetKind(before.type.name) ||
    !isWidgetKind(inserted.type.name) ||
    step.to !== step.from + before.nodeSize ||
    original.from < step.from ||
    original.to > step.to
  )
    return
  const pos = transaction.mapping.map(step.from, -1)
  const after = transaction.doc.nodeAt(pos)
  if (!after || after.type !== inserted.type) return
  if (transaction.selection.from >= pos && transaction.selection.to <= pos + after.nodeSize) return
  if (before.content.eq(after.content)) {
    const selection = original.toJSON()
    for (const edge of ['anchor', 'head'])
      if (typeof selection[edge] === 'number') selection[edge] += pos - step.from
    transaction.setSelection(Selection.fromJSON(transaction.doc, selection))
  } else {
    transaction.setSelection(Selection.near(transaction.doc.resolve(pos + 1)))
  }
}

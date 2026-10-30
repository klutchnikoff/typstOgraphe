import { writeFile } from 'node:fs/promises'
import type { Page, TestInfo } from '@playwright/test'

/** Text-only diagnostics, opt-in; no screenshot, trace archive or user document. */
export async function recordSelection(page: Page, label?: string) {
  await page.evaluate((label) => {
    const editor = label
      ? (
          document.querySelector(`[role="textbox"][aria-label="${label}"]`) as HTMLElement & {
            editor: import('@tiptap/core').Editor
          }
        ).editor
      : window.__spike!.editor
    const records: unknown[] = []
    const record = (phase: string, key?: string, stack?: string) => {
      const dom = document.getSelection()
      const position = (node: Node | null | undefined, offset: number | undefined) => {
        if (!node || !editor.view.dom.contains(node)) return null
        try {
          return editor.view.posAtDOM(node, offset || 0)
        } catch {
          return 'unmapped'
        }
      }
      records.push({
        phase,
        key,
        time: performance.now(),
        pm: editor.state.selection.toJSON(),
        marks: editor.state.storedMarks?.map((mark) => mark.toJSON()),
        editable: editor.isEditable,
        active: document.activeElement?.outerHTML.slice(0, 500),
        dom: [
          position(dom?.anchorNode, dom?.anchorOffset),
          position(dom?.focusNode, dom?.focusOffset),
        ],
        text: dom?.toString(),
        focused: editor.view.hasFocus(),
        stack,
      })
    }
    const collapse = Selection.prototype.collapse
    Selection.prototype.collapse = function (...args: Parameters<Selection['collapse']>) {
      record('collapse:before', undefined, new Error().stack)
      const result = collapse.apply(this, args)
      record('collapse:after')
      return result
    }
    const extend = Selection.prototype.extend
    Selection.prototype.extend = function (...args: Parameters<Selection['extend']>) {
      const result = extend.apply(this, args)
      record('extend')
      return result
    }
    editor.view.dom.addEventListener('keydown', (event) => record('keydown', event.key), true)
    editor.view.dom.addEventListener('keyup', (event) => record('keyup', event.key))
    document.addEventListener('selectionchange', () => record('native:selection'))
    editor.on('selectionUpdate', () => record('pm:selection'))
    editor.on('focus', () => record('pm:focus'))
    editor.on('blur', () => record('pm:blur'))
    editor.on('transaction', ({ transaction }) =>
      record(`pm:transaction:${JSON.stringify(transaction.steps.map((step) => step.toJSON()))}`),
    )
    Object.assign(window, { __selectionDiagnostic: records })
  }, label)
  return async (info: TestInfo) => {
    const records = await page.evaluate(() => Reflect.get(window, '__selectionDiagnostic'))
    const path = info.outputPath('selection-diagnostic.json')
    await writeFile(path, JSON.stringify(records, null, 2))
    await info.attach('selection-diagnostic', { path, contentType: 'application/json' })
  }
}

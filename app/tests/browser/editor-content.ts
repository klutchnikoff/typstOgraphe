import { expect, type Locator } from '@playwright/test'

/** Compare rendered content without ProseMirror's non-documentary widget decorations.
 * Keep document nodes and marks intact; never remove arbitrary text or user names.
 */
export async function expectEditorText(locator: Locator, expected: string | string[]) {
  await expect
    .poll(() =>
      locator.evaluateAll((nodes) =>
        nodes.map((node) => {
          const content = node.cloneNode(true) as Element
          content.querySelectorAll('.ProseMirror-widget').forEach((widget) => widget.remove())
          return content.textContent || ''
        }),
      ),
    )
    .toEqual(Array.isArray(expected) ? expected : [expected])
}

import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

for (const [context, html] of [
  ['texte simple', '<p>Texte<span data-footnote="Précision"></span></p>'],
  ['texte en gras', '<p><strong>Texte</strong><span data-footnote="Précision"></span></p>'],
  ['après une espace', '<p>Texte <span data-footnote="Précision"></span>Suite</p>'],
  ['après une espace insécable', '<p>Texte\u00a0<span data-footnote="Précision"></span>Suite</p>'],
  ['début de paragraphe', '<p><span data-footnote="Précision"></span>Suite</p>'],
] as const) {
  test(`curseur extérieur au badge dans ${context} : clic, flèches et saisie de part et d’autre`, async ({
    page,
  }) => {
    for (const trigger of ['click-before', 'left', 'right'] as const) {
      const pos = await page.evaluate(
        ({ html, trigger }) => {
          const e = window.__spike!.editor
          e.commands.setContent(html)
          let pos = 0
          e.state.doc.descendants((node, offset) => {
            if (node.type.name === 'footnote') pos = offset
          })
          e.commands.setTextSelection(pos + (trigger === 'left' ? 1 : 0))
          e.view.focus()
          return pos
        },
        { html, trigger },
      )
      if (trigger === 'click-before') {
        const point = await page.evaluate((pos) => {
          const e = window.__spike!.editor
          const rect = document.querySelector('.inline-object-label')!.getBoundingClientRect()
          const text = e.view.coordsAtPos(pos, -1)
          return {
            x: rect.left - Math.max(1, (rect.left - text.left) / 2),
            y: (text.top + text.bottom) / 2,
          }
        }, pos)
        await page.mouse.click(point.x, point.y)
      } else await page.keyboard.press(trigger === 'left' ? 'ArrowLeft' : 'ArrowRight')
      const expected = pos + (trigger === 'right' ? 1 : 0)
      await expect
        .poll(() =>
          page.evaluate(() => ({
            from: window.__spike!.editor.state.selection.from,
            to: window.__spike!.editor.state.selection.to,
          })),
        )
        .toEqual({ from: expected, to: expected })
      const geometry = await page.evaluate((after) => {
        const e = window.__spike!.editor,
          selection = window.getSelection()!
        const frame = document.querySelector('.inline-object-label')!.getBoundingClientRect()
        const range = selection.getRangeAt(0).getClientRects()[0]
        const caret = range || e.view.coordsAtPos(e.state.selection.from, after ? 1 : -1)
        return {
          edge: e.view.coordsAtPos(e.state.selection.from, 1).left,
          frameLeft: frame.left,
          inside: !!selection.anchorNode?.parentElement?.closest('.inline-object-label'),
          distance: after ? caret.left - frame.right : frame.left - caret.right,
          gap: frame.left - document.querySelector('.inline-object')!.getBoundingClientRect().left,
        }
      }, trigger === 'right')
      expect(geometry.inside).toBe(false)
      expect(geometry.distance).toBeGreaterThanOrEqual(-1)
      if (trigger !== 'right') {
        expect(geometry.edge - geometry.frameLeft).toBeLessThanOrEqual(1)
        if (context === 'début de paragraphe' || context.startsWith('après une espace'))
          expect(Math.abs(geometry.gap)).toBeLessThan(1)
        else expect(geometry.gap).toBeGreaterThan(2)
        expect(geometry.distance).toBeGreaterThanOrEqual(geometry.gap - 1)
      }
      await page.keyboard.insertText('ajout')
      const result = await page.evaluate(() => {
        const e = window.__spike!.editor
        const notes: { pos: number; text: string }[] = []
        e.state.doc.descendants((node, pos) => {
          if (node.type.name === 'footnote') notes.push({ pos, text: node.attrs.text })
        })
        return { notes, cursor: e.state.selection.from, text: e.getText() }
      })
      expect(result.notes).toEqual([
        { pos: pos + (trigger === 'right' ? 0 : 5), text: 'Précision' },
      ])
      expect(result.cursor).toBe(expected + 5)
      expect(result.text).not.toContain('\u2060')
      await expect(page.locator('.footnote-ref')).toHaveText('Note 1')
    }
  })
}

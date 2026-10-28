import { expect, test, type Page } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Un texte pour sélectionner normalement.</p><p></p>')
    editor.commands.setTextSelection(4)
    editor.view.focus()
  })
})

async function startDrag(page: Page) {
  const point = await page.evaluate(() => {
    const splitter = document.querySelector<HTMLElement>('.splitter')!
    splitter.addEventListener(
      'pointerdown',
      (e) => (splitter.dataset.pointer = String(e.pointerId)),
      { once: true },
    )
    const rect = splitter.getBoundingClientRect()
    const text = window.__spike!.editor.view.coordsAtPos(4)
    return { x: rect.left + rect.width / 2, y: (text.top + text.bottom) / 2 }
  })
  await page.mouse.move(point.x, point.y)
  await page.mouse.down()
  await expect(page.locator('html')).toHaveClass(/resizing-columns/)
  return point
}

async function selectText(page: Page) {
  const points = await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.view.focus()
    return [2, 9].map((pos) => {
      const rect = editor.view.coordsAtPos(pos)
      return { x: rect.left, y: (rect.top + rect.bottom) / 2 }
    })
  })
  await page.mouse.move(points[0].x, points[0].y)
  await page.mouse.down()
  await page.mouse.move(points[1].x, points[1].y, { steps: 5 })
  await page.mouse.up()
  await expect.poll(() => page.evaluate(() => window.getSelection()?.toString())).not.toBe('')
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.empty)).toBe(false)
}

test('redimensionner les colonnes ne sélectionne pas le texte et conserve le curseur', async ({
  page,
}) => {
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  const splitter = page.getByRole('separator', { name: 'Largeur des colonnes' })
  const initialWidth = Number(await splitter.getAttribute('aria-valuenow'))
  const point = await startDrag(page)
  for (const distance of [50, 100, 160]) {
    await page.mouse.move(point.x - distance, point.y, { steps: 5 })
    expect(await page.evaluate(() => window.getSelection()?.isCollapsed ?? true)).toBe(true)
    expect(
      await page.evaluate(() => {
        const selection = window.__spike!.editor.state.selection
        return { from: selection.from, to: selection.to }
      }),
    ).toEqual({ from: 4, to: 4 })
  }
  await page.mouse.up()
  await expect(page.locator('html')).not.toHaveClass(/resizing-columns/)
  const width = Number(await splitter.getAttribute('aria-valuenow'))
  expect(width).toBeLessThan(initialWidth)
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await splitter.press('ArrowRight')
  await expect(splitter).toHaveAttribute('aria-valuenow', String(width + 2))
  await selectText(page)
})

for (const interruption of ['pointercancel', 'lostpointercapture', 'blur'] as const) {
  test(`une interruption ${interruption} rétablit la sélection normale`, async ({ page }) => {
    const point = await startDrag(page)
    await page.mouse.move(point.x - 100, point.y, { steps: 5 })
    await page.evaluate((interruption) => {
      const splitter = document.querySelector<HTMLElement>('.splitter')!
      const pointerId = Number(splitter.dataset.pointer)
      if (interruption === 'blur') window.dispatchEvent(new Event('blur'))
      else if (interruption === 'lostpointercapture') splitter.releasePointerCapture(pointerId)
      else splitter.dispatchEvent(new PointerEvent('pointercancel', { pointerId }))
    }, interruption)
    await page.mouse.move(point.x - 110, point.y)
    await page.mouse.up()
    await expect(page.locator('html')).not.toHaveClass(/resizing-columns/)
    await selectText(page)
    await startDrag(page)
    await page.mouse.up()
    await expect(page.locator('html')).not.toHaveClass(/resizing-columns/)
  })
}

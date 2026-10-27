import { expect, test, type Page } from '@playwright/test'

async function roots(page: Page) {
  return page.evaluate(() =>
    window.__spike!.editor.state.doc.content.content.map((node) => [
      node.type.name,
      node.textContent,
    ]),
  )
}
test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('#pages .preview-page').first()).toBeVisible()
})
test('insertion commune : tableau au début, milieu, fin ou après un retour, et tous les autres widgets', async ({
  page,
}, info) => {
  test.setTimeout(120000)
  const cases = [
    ['insert.table', 'table', 1],
    ['insert.table', 'table', 6],
    ['insert.table', 'table', 11],
    ['insert.table', 'table', 7, true],
    ['block.bullet', 'bulletList', 6],
    ['block.numbered', 'orderedList', 6],
    ['block.description', 'descriptionList', 6],
    ['block.quote', 'quote', 6],
    ['letter.insert_postscript', 'postscript', 6],
    ['letter.insert_attachment', 'attachment', 6],
    ['insert.image', 'figure', 6],
  ] as const
  for (const [id, kind, pos, soft] of cases) {
    await page.evaluate(
      ({ pos, soft }) => {
        const e = window.__spike!.editor
        e.commands.setContent({
          type: 'doc',
          content: [
            {
              type: 'paragraph',
              content: soft
                ? [
                    { type: 'text', text: 'Avant' },
                    { type: 'softBreak' },
                    { type: 'text', text: 'Après' },
                  ]
                : [{ type: 'text', text: 'AvantAprès' }],
            },
          ],
        })
        e.commands.setTextSelection(pos)
        e.view.focus()
      },
      { pos, soft },
    )
    const before = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
    if (id.startsWith('block.') && kind !== 'quote')
      await page.getByRole('button', { name: 'Listes', exact: true }).click()
    if (id.startsWith('letter.'))
      await page.getByRole('button', { name: 'Fin de lettre', exact: true }).click()
    if (kind === 'figure') {
      const choosing = page.waitForEvent('filechooser')
      await page.locator(`[data-command="${id}"]`).click()
      await (
        await choosing
      ).setFiles({
        name: 'insertion.svg',
        mimeType: 'image/svg+xml',
        buffer: Buffer.from(
          '<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40"><rect width="80" height="40" fill="#809890"/></svg>',
        ),
      })
    } else await page.locator(`[data-command="${id}"]`).click()
    const expected =
      pos === 1
        ? [
            [kind, ''],
            ['paragraph', ''],
            ['paragraph', 'AvantAprès'],
          ]
        : pos === 11
          ? [
              ['paragraph', 'AvantAprès'],
              [kind, ''],
              ['paragraph', ''],
            ]
          : [
              ['paragraph', 'Avant'],
              [kind, ''],
              ['paragraph', ''],
              ['paragraph', 'Après'],
            ]
    await expect.poll(() => roots(page)).toEqual(expected)
    await expect
      .poll(() =>
        page.evaluate((kind) => {
          const $from = window.__spike!.editor.state.selection.$from
          return Array.from(
            { length: $from.depth + 1 },
            (_, depth) => $from.node(depth).type.name,
          ).includes(kind)
        }, kind),
      )
      .toBe(true)
    await expect(page.locator('#render-status')).toHaveText(/^\d+ pages? ·/)
    if (kind === 'table')
      await expect
        .poll(() =>
          page.locator('.widget-table').evaluate((widget) => {
            const header = widget.querySelector('.widget-bar')!.getBoundingClientRect()
            const table = widget.querySelector('table')!.getBoundingClientRect()
            return Math.abs(table.top - header.bottom)
          }),
        )
        .toBeLessThan(2)
    const line = page.locator('.tiptap > p').filter({ hasText: /^$/ })
    await expect(line).toHaveCount(1)
    await line.click()
    await expect
      .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.depth))
      .toBe(1)
    if (kind === 'table') await page.keyboard.type('Reprise')
    await expect(page.locator('.tiptap > p').filter({ hasText: /^Reprise$/ })).toHaveCount(1)
    await page.getByRole('button', { name: 'Annuler', exact: true }).click()
    await expect.poll(() => roots(page)).toEqual(expected)
    await page.getByRole('button', { name: 'Annuler', exact: true }).click()
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
  }
})

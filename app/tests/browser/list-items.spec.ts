import { expect, test, type Page } from '@playwright/test'
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
async function ready(page: Page, id = 'main') {
  await page.goto('/?client=' + id)
  await page.waitForFunction(() => window.__spike?.ready)
}
async function at(page: Page, text: string, end = false) {
  await page.evaluate(
    ({ text, end }) => {
      const e = window.__spike!.editor
      e.state.doc.descendants((n, pos) => {
        if (n.isTextblock && n.textContent === text)
          e.commands.setTextSelection(pos + 1 + (end ? n.content.size : 0))
      })
      e.view.focus()
    },
    { text, end },
  )
}
test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await ready(page)
  await page.evaluate(async () => {
    const s = window.__spike!
    s.editor.commands.setContent(
      '<p>Avant</p><ul><li><p>Premier</p><p>Suite</p></li><li><p><strong>Deuxième</strong><span data-footnote="Note conservée"></span></p></li><li><p>Voisin</p></li></ul><p>Après</p>',
    )
    await s.provider.flush()
    await s.provider.clearHistory()
  })
})
test('Suppr joint les items sans quitter le widget ; l’annulation restaure contenu, curseur et seconde fenêtre', async ({
  page,
  context,
}) => {
  const peer = await context.newPage()
  await ready(peer, 'list-item-peer')
  await at(page, 'Suite', true)
  await page.keyboard.press('Delete')
  await expect(body(page).locator('li')).toHaveCount(2)
  await expect(body(page).locator('li').first().locator('p')).toHaveText([
    'Premier',
    'SuiteDeuxièmeNote 1',
  ])
  await expect(body(peer).locator('li')).toHaveCount(2)
  await expect(body(page).locator('strong')).toHaveText('Deuxième')
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset)).toBe(
    5,
  )
  await page.evaluate(() => window.__spike!.provider.flush())
  await body(page).press('ControlOrMeta+z')
  await expect(body(peer).locator('li')).toHaveCount(3)
  expect(
    await page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.textContent),
  ).toBe('Suite')
  await body(page).press('ControlOrMeta+Shift+z')
  await expect(body(peer).locator('li')).toHaveCount(2)
  await peer.close()
})
test('Entrée sépare un item au curseur sans perdre la note ni les paragraphes suivants', async ({
  page,
}) => {
  await at(page, 'Premier')
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('Enter')
  await expect(body(page).locator('li')).toHaveCount(4)
  await expect(body(page).locator('li').first()).toHaveText('Pr')
  await expect(body(page).locator('li').nth(1).locator('p')).toHaveText(['emier', 'Suite'])
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset)).toBe(
    0,
  )
  await page.keyboard.press('Backspace')
  await expect(body(page).locator('li')).toHaveCount(3)
  await expect(body(page).locator('.footnote-ref')).toHaveText('Note 1')
})
test('un item avec note et paragraphe vide ne sort pas comme un item entièrement vide', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const s = window.__spike!
    s.editor.commands.setContent(
      '<ul><li><p><span data-footnote="Ne pas perdre"></span></p><p></p></li></ul>',
    )
    await s.provider.flush()
    s.editor.state.doc.descendants((n, pos, parent) => {
      if (parent?.type.name === 'listItem' && n.type.name === 'paragraph' && !n.content.size)
        s.editor.commands.setTextSelection(pos + 1)
    })
    s.editor.view.focus()
  })
  await page.keyboard.press('Enter')
  await expect(body(page).locator('li')).toHaveCount(2)
  await expect(body(page).locator('.footnote-ref')).toHaveText('Note 1')
  await page.keyboard.press('Enter')
  await expect(body(page).locator('li')).toHaveCount(1)
  await expect(body(page).locator('.footnote-ref')).toHaveText('Note 1')
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.$from.depth)).toBe(1)
})

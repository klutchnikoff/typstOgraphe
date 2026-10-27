import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('Entrée réutilise le retour suivant et maintient une seule ligne finale éditable', async ({
  page,
}) => {
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Avant<br data-soft-break>Après</p>')
    editor.commands.setTextSelection(6)
    editor.view.focus()
  })
  await page.keyboard.press('Enter')
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(7)
  await expect(page.locator('.tiptap br[data-soft-break]')).toHaveCount(1)
  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('Shift+Enter')
  await expect(page.locator('.tiptap br[data-soft-break]')).toHaveCount(1)
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(7)
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setTextSelection(editor.state.doc.content.size - 1)
    editor.view.focus()
  })
  await page.keyboard.type('Suite')
  await expect(page.locator('.tiptap > p')).toHaveText(['AvantAprès', 'Suite', ''])
  await expect(page.locator('.tiptap > p').last()).toBeEmpty()
  await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap > p')).toHaveCount(2)
  await page.keyboard.press('Enter')
  for (let i = 0; i < 4; i++) await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap > p')).toHaveText(['AvantAprès', 'Suite', ''])
  await expect(page.locator('.tiptap .paragraph-marker').last()).toHaveText('¶')
  await page.keyboard.type('Fin')
  await expect(page.locator('.tiptap > p')).toHaveText(['AvantAprès', 'Suite', 'Fin', ''])
})

test('aucune ligne vide initiale, même après collage, et le document vide reste utilisable', async ({
  page,
}) => {
  for (const html of ['<p>Premier</p>', '<h1>Titre</h1>', '<p></p>']) {
    await page.evaluate((html) => {
      const e = window.__spike!.editor
      e.commands.setContent(html)
      e.commands.setTextSelection(1)
      e.view.focus()
    }, html)
    const before = await page.evaluate(() => window.__spike!.editor.getJSON())
    for (const key of ['Enter', 'Enter', 'Shift+Enter']) await page.keyboard.press(key)
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    await page.keyboard.insertText('Début ')
    expect(
      await page.evaluate(() => window.__spike!.editor.state.doc.firstChild!.textContent),
    ).toMatch(/^Début /)
  }
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent('<p></p><p><br data-soft-break><strong>Corps</strong></p>')
  })
  await expect(page.locator('.tiptap > p')).toHaveText(['Corps', ''])
  await expect(page.locator('.tiptap > p').first().locator('strong')).toHaveText('Corps')
})

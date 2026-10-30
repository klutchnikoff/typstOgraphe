import { expect, test } from '@playwright/test'
import { confirmNewDocument } from './document-lifecycle'

test.beforeEach(async ({ request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
})
for (const key of ['Backspace', 'Delete']) {
  test(`la fusion ${key} restaure les blocs et le curseur lors d’annuler/rétablir`, async ({
    page,
    context,
  }) => {
    await page.goto('/?client=main')
    await page.waitForFunction(() => window.__spike?.ready)
    const body = page.getByRole('textbox', { name: 'Corps du document', exact: true })
    await page.evaluate(async () => {
      const { editor, provider } = window.__spike!
      editor.commands.setContent('<p>Avant😀</p><h2>Après</h2>')
      await provider.flush()
      await provider.clearHistory()
    })
    const peer = await context.newPage()
    await peer.goto('/?client=join-peer')
    await peer.waitForFunction(() => window.__spike?.ready)
    await body.focus()
    const original = key === 'Backspace' ? 10 : 8
    await page.evaluate((pos) => window.__spike!.editor.commands.setTextSelection(pos), original)
    await page.keyboard.press(key)
    await expect(body).toContainText('Avant😀Après')
    await expect
      .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.from))
      .toBe(8)
    await page.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    await expect(
      peer.getByRole('textbox', { name: 'Corps du document', exact: true }),
    ).toContainText('Avant😀Après')
    await page.keyboard.press('ControlOrMeta+z')
    await expect(body.locator('h2')).toHaveText('Après')
    await expect
      .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.from))
      .toBe(original)
    await expect(body).toBeFocused()
    await page.keyboard.press('ControlOrMeta+Shift+z')
    await expect(body.locator('h2')).toHaveCount(0)
    await expect
      .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.from))
      .toBe(8)
    await peer.close()
  })
}

test('la même fusion et son historique restent dans le résumé riche actif', async ({ page }) => {
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await confirmNewDocument(page)
  await page.locator('#metadata-tab-abstracts').click()
  const abstract = page.getByRole('textbox', { name: 'Résumé', exact: true })
  await abstract.fill('Avant')
  await abstract.press('End')
  await abstract.press('Enter')
  await abstract.press('Enter')
  await page.keyboard.type('Après')
  await expect(abstract.locator('p')).toHaveCount(2)
  await page.evaluate(async () => {
    await window.__spike!.provider.clearHistory()
  })
  // Home scrolls without moving the caret in macOS WebKit. Exercise the
  // boundary using actual, portable cursor movement in both engines.
  for (let i = 0; i < 'Après'.length; i++) await abstract.press('ArrowLeft')
  await expect.poll(() => page.evaluate(() => document.getSelection()?.focusOffset)).toBe(0)
  await abstract.press('Backspace')
  await expect(abstract.locator('p')).toHaveCount(1)
  await expect(abstract).toHaveText('AvantAprès')
  await abstract.press('ControlOrMeta+z')
  await expect(abstract.locator('p')).toHaveCount(2)
  await expect(abstract).toBeFocused()
  await expect
    .poll(() =>
      page.evaluate(() => {
        const s = document.getSelection()
        return { text: s?.focusNode?.textContent, offset: s?.focusOffset }
      }),
    )
    .toEqual({ text: 'Après', offset: 0 })
  await abstract.press('ControlOrMeta+Shift+z')
  await expect(abstract.locator('p')).toHaveCount(1)
  await expect(abstract).toHaveText('AvantAprès')
  await expect(abstract).toBeFocused()
})

import { expect, test, type Page } from '@playwright/test'

const settleFocus = (page: Page) =>
  page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  )
test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})
for (const trigger of ['palette-keyboard', 'palette-pointer', 'toolbar'] as const) {
  test(`note : focus stable dans le champ depuis ${trigger}, validation et retour au texte`, async ({
    page,
  }) => {
    await page.evaluate(() => {
      const editor = window.__spike!.editor
      editor.commands.setContent('<p>Test des notes</p>')
      editor.commands.setTextSelection(5)
      editor.view.focus()
    })
    if (trigger === 'toolbar') await page.locator('[data-command="insert.footnote"]').click()
    else {
      await page.keyboard.press('Control+Shift+P')
      const query = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
      await query.fill('note de bas')
      const choice = page.getByRole('option')
      await expect(choice).toHaveCount(1)
      if (trigger === 'palette-keyboard') await query.press('Enter')
      else await choice.click()
    }
    const field = page.getByRole('textbox', { name: 'Texte de la note', exact: true })
    await settleFocus(page)
    await expect(field).toBeFocused()
    await expect(field).toHaveAccessibleName('Texte de la note')
    await expect(page.locator('#properties label[for="parameter-text"]')).toBeHidden()
    await expect(page.locator('#property-title')).toHaveText('Note de bas de page')
    // Typing must reach the focused field; fill() would mask a focus regression.
    await page.keyboard.insertText('Une précision utile.')
    await expect(field).toHaveValue('Une précision utile.')
    expect(
      await page.evaluate(() => window.__spike!.editor.state.doc.firstChild!.textContent),
    ).toBe('Test des notes')
    await page.keyboard.press('Enter')
    await expect(page.locator('#properties')).toBeHidden()
    await expect(page.locator('.tiptap')).toBeFocused()
    expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(6)
    await page.keyboard.insertText(' ici')
    expect(
      await page.evaluate(() => window.__spike!.editor.state.doc.firstChild!.textContent),
    ).toBe('Test ici des notes')
    for (const shortcut of ['Control+Enter', 'Meta+Enter']) {
      await page.evaluate(() => {
        const editor = window.__spike!.editor
        editor.commands.setTextSelection(6)
        editor.view.focus()
      })
      await page.keyboard.press(shortcut)
      await settleFocus(page)
      await expect(field).toBeFocused()
      await expect(field).toHaveValue('Une précision utile.')
      await page.keyboard.press('Escape')
      await expect(page.locator('.tiptap')).toBeFocused()
      expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(6)
    }
  })
}

test('lien : le même contrat garde le focus et restitue la sélection après validation ou annulation', async ({
  page,
}) => {
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Lien utile</p>')
    editor.commands.setTextSelection({ from: 1, to: 5 })
    editor.view.focus()
  })
  await page.keyboard.press('Control+Shift+P')
  const query = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
  await query.fill('lien')
  await query.press('Enter')
  const field = page.getByRole('textbox', { name: 'Adresse du lien', exact: true })
  await settleFocus(page)
  await expect(field).toBeFocused()
  await page.keyboard.insertText('https://example.org')
  await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap')).toBeFocused()
  expect(
    await page.evaluate(() => ({
      from: window.__spike!.editor.state.selection.from,
      to: window.__spike!.editor.state.selection.to,
    })),
  ).toEqual({ from: 1, to: 5 })
  await expect(page.locator('.tiptap a')).toHaveAttribute('href', 'https://example.org')
  await page.keyboard.press('Control+Enter')
  await settleFocus(page)
  await expect(field).toBeFocused()
  await page.keyboard.insertText('/draft')
  await page.evaluate(() => window.__spike!.editor.commands.insertContentAt(1, 'Pré '))
  await page.keyboard.press('Escape')
  await expect(page.locator('.tiptap')).toBeFocused()
  expect(
    await page.evaluate(() => ({
      from: window.__spike!.editor.state.selection.from,
      to: window.__spike!.editor.state.selection.to,
    })),
  ).toEqual({ from: 5, to: 9 })
  await expect(page.locator('.tiptap a').last()).toHaveAttribute('href', 'https://example.org')
})

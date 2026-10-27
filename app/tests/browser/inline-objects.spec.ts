import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('le badge est un objet accessible ; son contexte reste distinct du mot incorrect voisin', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'courrrier' },
            { type: 'footnote', attrs: { text: 'Une précision.' } },
            { type: 'text', text: ' suite' },
          ],
        },
      ],
    })
    e.commands.setTextSelection(4)
    e.view.focus()
  })
  const note = page.getByRole('button', {
    name: 'Note de bas de page : Une précision.',
    exact: true,
  })
  await expect(note).toHaveText('Note')
  await expect(note).toHaveAttribute('contenteditable', 'false')
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
  await page.keyboard.press('Control+Enter')
  await expect(page.locator('#proofing-band')).toBeVisible()
  await page.keyboard.press('Escape')
  // Before/after a reference, and when explicitly selected, target the note.
  for (const position of [10, 11, 'selected'] as const) {
    await page.evaluate((pos) => {
      const e = window.__spike!.editor
      if (pos === 'selected') e.commands.setNodeSelection(10)
      else e.commands.setTextSelection(pos)
      e.view.focus()
    }, position)
    await page.keyboard.press('Control+Enter')
    await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
    await expect(page.locator('#proofing-band')).toHaveCount(0)
    await page.keyboard.press('Escape')
  }
  await note.click()
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
  await page.keyboard.press('Escape')
  expect(
    await page.evaluate(
      () => window.__spike!.editor.state.selection.to - window.__spike!.editor.state.selection.from,
    ),
  ).toBe(1)
  // Keyboard access to the badge uses the same object selection and band.
  await note.focus()
  await page.keyboard.press('Space')
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
})

test('une note modifiée ailleurs ne peut pas être supprimée depuis ses anciens paramètres', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent('<p>Avant<span data-footnote="Ancien texte"></span> après</p>')
  })
  await page.locator('.footnote-ref').click()
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'footnote')
        e.view.dispatch(e.state.tr.setNodeMarkup(pos, undefined, { text: 'Modification distante' }))
    })
  })
  await page.locator('#remove-block').click()
  await expect(page.locator('#property-error')).toContainText('modifié ailleurs')
  await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'settings')
  await expect(page.locator('.footnote-ref')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await page.locator('.footnote-ref').click()
  await expect(page.getByLabel('Texte de la note', { exact: true })).toHaveValue(
    'Modification distante',
  )
})

for (const context of ['prose', 'quote'] as const) {
  test(`suppression protégée d’une note dans ${context}, annulation, confirmation et undo`, async ({
    page,
  }) => {
    await page.evaluate((kind) => {
      const e = window.__spike!.editor
      const p = {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'Avant' },
          { type: 'footnote', attrs: { text: 'À conserver.' } },
          { type: 'text', text: ' après' },
        ],
      }
      e.commands.setContent({
        type: 'doc',
        content:
          kind === 'quote' ? [{ type: 'quote', content: [p, { type: 'quoteAttribution' }] }] : [p],
      })
      e.view.focus()
    }, context)
    const before = await page.evaluate(() => window.__spike!.editor.getJSON())
    for (const key of ['Backspace', 'Delete', 'Control+Backspace', 'Meta+Backspace']) {
      await page.evaluate((key) => {
        const e = window.__spike!.editor
        e.state.doc.descendants((node, pos) => {
          if (node.type.name === 'footnote')
            e.commands.setTextSelection(pos + (key === 'Delete' ? 0 : 1))
        })
        e.view.focus()
      }, key)
      await page.keyboard.press(key)
      await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'remove')
      await expect(page.locator('#property-title')).toHaveText('Supprimer la note de bas de page')
      expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
      await page.keyboard.press('Escape')
      await expect(page.locator('.tiptap')).toBeFocused()
      expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    }
    // The properties button goes through the very same confirmation.
    await page.locator('.footnote-ref').click()
    await page.locator('#remove-block').click()
    await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'remove')
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    await page.keyboard.press('Enter')
    await expect(page.locator('.footnote-ref')).toHaveCount(0)
    await expect(page.locator('.tiptap')).toBeFocused()
    const body = context === 'quote' ? '.tiptap blockquote p' : '.tiptap > p:first-child'
    await expect(page.locator(body)).toContainText('Avant après')
    await page.keyboard.press('Control+z')
    await expect(page.locator('.footnote-ref')).toHaveCount(1)
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  })
}

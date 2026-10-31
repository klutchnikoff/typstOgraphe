import { expect, test, type Page } from '@playwright/test'
async function ready(page: Page) {
  await page.request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Texte original de la version.' }] },
        { type: 'paragraph' },
      ],
    })
    await provider.flush()
  })
}
async function versions(page: Page) {
  await page.locator('.side-rail [data-command="ui.versions"]').click()
}
async function capture(page: Page, name: string) {
  await page.getByRole('button', { name: 'Créer une version', exact: true }).click()
  await page.locator('dialog [name=name]').fill(name)
  await page.locator('dialog [name=tags]').fill('brouillon, première étape')
  await page.locator('dialog [name=name]').press('Enter')
  await expect(page.locator('dialog[open]')).toHaveCount(0)
  await expect(
    page.locator('.versions-groups [data-version]').filter({ hasText: name }),
  ).toBeVisible()
}
test('Versions preserves live writing through read-only consultation, preview, relabelling and confirmed deletion', async ({
  page,
}) => {
  await ready(page)
  await versions(page)
  await capture(page, 'Première version')
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [{ type: 'text', text: 'Texte courant modifié après la capture.' }],
        },
        { type: 'paragraph' },
      ],
    })
    await provider.flush()
  })
  await page.locator('.versions-groups [data-version]').click()
  const dialog = page.locator('.version-dialog[open]')
  await expect(dialog.locator('.version-prose')).toContainText('Texte original de la version.')
  await expect(dialog.locator('.version-prose')).not.toContainText('Texte courant modifié')
  await expect(dialog.locator('[contenteditable=true]')).toHaveCount(0)
  const box = await dialog.boundingBox(),
    viewport = page.viewportSize()!
  expect(box!.width / viewport.width).toBeCloseTo(0.9, 1)
  expect(box!.height / viewport.height).toBeCloseTo(0.9, 1)
  await dialog.getByRole('tab', { name: 'Contenu', exact: true }).focus()
  await page.keyboard.press('ArrowRight')
  await expect(dialog.getByRole('tab', { name: 'Aperçu', exact: true })).toHaveAttribute(
    'aria-selected',
    'true',
  )
  await expect
    .poll(() =>
      dialog
        .locator('.version-page')
        .evaluateAll(
          (images) =>
            images.length > 0 &&
            images.every(
              (image) =>
                (image as HTMLImageElement).complete &&
                (image as HTMLImageElement).naturalWidth > 0,
            ),
        ),
    )
    .toBe(true)
  await dialog
    .getByRole('button', { name: 'Modifier les informations de la version', exact: true })
    .click()
  await expect(dialog.locator('[name=name]')).toBeFocused()
  await dialog.locator('[name=name]').fill('Version relue')
  await dialog.locator('[name=name]').press('Enter')
  await expect(dialog.getByRole('heading', { name: 'Version relue', exact: true })).toBeVisible()
  await expect(dialog.locator('.properties')).toHaveCount(0)
  await dialog.getByRole('button', { name: 'Supprimer cette version', exact: true }).click()
  await expect(dialog.locator('.properties .primary')).toHaveText('Annuler')
  await page.keyboard.press('Enter')
  await expect(dialog.locator('.properties')).toHaveCount(0)
  await dialog.getByRole('button', { name: 'Supprimer cette version', exact: true }).click()
  await page.keyboard.press('Escape')
  await expect(dialog).toBeVisible()
  await expect(dialog.locator('.properties')).toHaveCount(0)
  await dialog.getByRole('button', { name: 'Supprimer cette version', exact: true }).click()
  await dialog
    .locator('.properties')
    .getByRole('button', { name: 'Supprimer', exact: true })
    .click()
  await expect(dialog).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Créer une version', exact: true })).toBeFocused()
  await expect(page.locator('.versions-groups [data-version]')).toHaveCount(0)
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain(
    'Texte courant modifié après la capture.',
  )
})
test('Escape closes consultation and returns to its exact version row', async ({ page }) => {
  await ready(page)
  await versions(page)
  await capture(page, 'Point de retour')
  const row = page.locator('.versions-groups [data-version]')
  await row.click()
  await expect(page.locator('.version-prose')).toContainText('Texte original')
  await page.keyboard.press('Escape')
  await expect(page.locator('dialog[open]')).toHaveCount(0)
  await expect(row).toBeFocused()
})

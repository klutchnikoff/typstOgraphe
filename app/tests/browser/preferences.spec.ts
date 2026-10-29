import { expect, test } from '@playwright/test'

test('une largeur modifiée pendant l’enregistrement des préférences ne rétablit pas les anciens réglages', async ({
  page,
  request,
}) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  let release!: () => void
  let saved!: () => void
  const held = new Promise<void>((resolve) => (release = resolve))
  const written = new Promise<void>((resolve) => (saved = resolve))
  let first = true
  await page.route('**/__spike/invoke', async (route) => {
    const args = route.request().postDataJSON()
    if (args.command === 'preferences_save' && first) {
      first = false
      const response = await route.fetch()
      saved()
      await held
      await route.fulfill({ response })
    } else await route.continue()
  })
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  await page.getByLabel('Apparence', { exact: true }).selectOption('dark')
  await page.getByLabel('Taille', { exact: true }).fill('21')
  await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click()
  await written
  const resized = page.waitForResponse((response) => {
    if (!response.url().endsWith('/__spike/invoke') || response.request().method() !== 'POST')
      return false
    const args = response.request().postDataJSON()
    return args.command === 'preferences_save' && args.args.preferences.editor_fraction === 0.52
  })
  const splitter = page.getByRole('separator', { name: 'Largeur des colonnes' })
  await splitter.focus()
  await splitter.press('ArrowRight')
  release()
  const response = await resized
  const preferences = response.request().postDataJSON().args.preferences
  expect(preferences.appearance).toBe('dark')
  expect(preferences.size).toBe(21)
  await expect(page.locator('html')).toHaveAttribute('data-appearance', 'dark')
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('html')).toHaveAttribute('data-appearance', 'dark')
  await expect(page.locator('.tiptap')).toHaveCSS('font-size', '21px')
  await expect(splitter).toHaveAttribute('aria-valuenow', '52')
})

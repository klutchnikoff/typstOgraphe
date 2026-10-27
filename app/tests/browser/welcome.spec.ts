import { expect, test } from '@playwright/test'

test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})

test('accueil centré, trois actions immédiates et animation non rejouée au retour', async ({
  page,
}, testInfo) => {
  await page.goto('/?client=main&welcome=1')
  await page.waitForFunction(() => window.__spike?.ready)
  const welcome = page.locator('.welcome')
  const brand = welcome.getByRole('img', { name: 'typstOgraphe', exact: true })
  const buttons = welcome.getByRole('button')
  await expect(buttons).toHaveCount(3)
  await expect(welcome).not.toContainText('Le contenu d’abord.')
  await expect(page.locator('#resume-document')).toBeVisible()
  const bounds = await buttons.evaluateAll((elements) =>
    elements.map((element) => {
      const { x, y, width, height } = element.getBoundingClientRect()
      return { center: x + width / 2, y, width, height }
    }),
  )
  for (const [index, button] of bounds.entries()) {
    expect(Math.abs(button.center - bounds[0].center)).toBeLessThan(1)
    expect(button.width).toBe(bounds[0].width)
    if (index) expect(button.y).toBeGreaterThan(bounds[index - 1].y + bounds[index - 1].height)
  }
  await expect(brand).toHaveAttribute('data-state', 'writing')
  // The new-document dialog is usable while the signature is still being drawn.
  await page.locator('#create-document').click()
  await expect(page.locator('dialog')).toBeVisible()
  await page.getByRole('button', { name: 'Créer', exact: true }).click()
  await expect(welcome).toBeHidden()
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.close"]').click()
  await expect(welcome).toBeVisible()
  await expect(brand).toHaveAttribute('data-state', 'settled')
  expect(
    await brand.evaluate((element) =>
      element
        .getAnimations({ subtree: true })
        .every((animation) => animation.playState === 'finished'),
    ),
  ).toBe(true)
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(brand).toHaveAttribute('data-state', 'settled')
})

test('accueil sans mouvement et actions accessibles à la taille minimale', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await page.setViewportSize({ width: 800, height: 600 })
  await page.goto('/?client=main&welcome=1')
  await page.waitForFunction(() => window.__spike?.ready)
  const brand = page.locator('.welcome-brand')
  await expect(brand).toHaveAttribute('data-state', 'settled')
  expect(await brand.evaluate((element) => element.getAnimations({ subtree: true }).length)).toBe(0)
  const buttons = page.locator('.welcome-actions button')
  await expect(buttons).toHaveCount(3)
  for (const button of await buttons.all()) await expect(button).toBeInViewport()
  await expect(page.locator('#resume-document')).toBeDisabled()
  await expect(page.locator('#create-document')).toBeEnabled()
  await expect(page.locator('#open-document')).toBeEnabled()
})

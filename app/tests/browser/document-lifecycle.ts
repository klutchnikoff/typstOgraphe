import type { Page } from '@playwright/test'

/** Creation reloads the view. The old page can still report ready while its
 * replacement is loading, so observe navigation before reading the new owner.
 */
export async function confirmNewDocument(page: Page) {
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page.getByRole('button', { name: 'Créer', exact: true }).click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
}

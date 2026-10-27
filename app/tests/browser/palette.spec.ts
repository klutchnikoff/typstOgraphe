import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('les flèches parcourent les commandes filtrées sans quitter la recherche et Entrée lance le choix actif', async ({
  page,
}) => {
  await page.evaluate(() => {
    window.__spike!.editor.commands.setContent('<p>Un titre à appliquer</p>')
    window.__spike!.editor.commands.focus('start')
  })
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  await page.keyboard.press('Control+Shift+P')
  const input = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
  await expect(input).toBeFocused()
  await input.fill('Titre')
  const options = page.getByRole('listbox', { name: 'Commandes', exact: true }).getByRole('option')
  await expect(options).toHaveCount(3)
  const selected = page.getByRole('option', { selected: true })
  await expect(selected).toHaveText('Titre 1')
  await input.press('ArrowDown')
  await expect(selected).toHaveText('Titre 2')
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('Titre')
  await expect(input).toHaveAttribute('aria-activedescendant', (await selected.getAttribute('id'))!)
  await input.press('ArrowUp')
  await expect(selected).toHaveText('Titre 1')
  await input.press('ArrowUp')
  await expect(selected).toHaveText('Titre 3')
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await input.fill('Titre 2')
  await expect(selected).toHaveText('Titre 2')
  await input.press('Enter')
  await expect(page.locator('.side-panel')).toBeHidden()
  await expect(page.locator('.tiptap h2')).toHaveText('Un titre à appliquer')
  await expect(page.locator('.tiptap')).toBeFocused()
})

test('la recherche reste visible pendant le défilement, sans action pour un résultat vide et fermeture avec Échap', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Commandes', exact: true }).click()
  const input = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
  const results = page.getByRole('listbox', { name: 'Commandes', exact: true })
  await expect(input).toBeFocused()
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  // The first enabled result is initially active; Up wraps to the last enabled command.
  await input.press('ArrowUp')
  const selected = results.getByRole('option', { selected: true })
  await expect(selected).toBeInViewport({ ratio: 1 })
  await expect(input).toBeInViewport({ ratio: 1 })
  await expect(input).toBeFocused()
  expect(await results.evaluate((element) => element.scrollTop)).toBeGreaterThan(0)
  await input.fill('e')
  await expect(selected).toBeInViewport({ ratio: 1 })
  expect(await results.evaluate((element) => element.scrollTop)).toBe(0)
  await input.fill('zz-aucune-commande-zz')
  await expect(results.getByRole('option')).toHaveCount(0)
  await expect(
    page.getByRole('status').filter({ hasText: 'Aucune commande ne correspond.' }),
  ).toBeVisible()
  await expect(input).not.toHaveAttribute('aria-activedescendant')
  await input.press('ArrowDown')
  await input.press('Enter')
  await expect(input).toBeFocused()
  await expect(page.locator('.side-panel')).toBeVisible()
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await input.press('Escape')
  await expect(page.locator('.side-panel')).toBeHidden()
  await expect(page.locator('.tiptap')).toBeFocused()
})

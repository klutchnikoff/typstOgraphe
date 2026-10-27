import { expect, test, type Page } from '@playwright/test'

const contents = (page: Page) => page.evaluate(() => window.__spike!.editor.getJSON())
test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('menus : les flèches déplacent le choix sans appliquer de commande et Échap restitue le focus', async ({
  page,
}) => {
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Le titre du courrier</p>')
    editor.commands.focus('start')
  })
  const before = await contents(page)
  const trigger = page.getByRole('button', { name: 'Titres', exact: true })
  await trigger.focus()
  await trigger.press('ArrowDown')
  const menu = trigger.locator('..').getByRole('menu')
  await expect(menu.getByRole('menuitem').first()).toBeFocused()
  await page.keyboard.press('ArrowDown')
  await expect(menu.getByRole('menuitem', { name: 'Titre 1', exact: true })).toBeFocused()
  expect(await contents(page)).toEqual(before)
  await page.keyboard.press('Escape')
  await expect(trigger).toBeFocused()
  await expect(menu).toBeHidden()
  await trigger.press('ArrowDown')
  await page.keyboard.press('ArrowDown')
  await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap h1')).toHaveText('Le titre du courrier')
})

test('onglets : les flèches changent de section et Échap ferme les informations en revenant au déclencheur', async ({
  page,
}) => {
  const tabs = page.locator('#metadata-tabs button')
  await tabs.first().click()
  await expect(page.locator('#metadata-content')).toBeVisible()
  await tabs.first().focus()
  await page.keyboard.press('ArrowRight')
  await expect(tabs.nth(1)).toBeFocused()
  await expect(tabs.nth(1)).toHaveAttribute('aria-pressed', 'true')
  const field = page
    .locator('#metadata-content input, #metadata-content select, #metadata-content textarea')
    .first()
  await field.focus()
  await field.press('Escape')
  await expect(page.locator('#metadata-content')).toBeHidden()
  await expect(tabs.nth(1)).toBeFocused()
})

test('orthographe : les suggestions tardives ne volent pas le focus et les flèches ne corrigent pas le texte', async ({
  page,
}) => {
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command !== 'spelling_suggestions') return route.continue()
    await route.fulfill({ json: { result: ['courrier', 'courriers'] } })
  })
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Ce courrrier est prêt.</p>')
    editor.commands.setTextSelection(6)
    editor.commands.focus()
  })
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
  const before = await contents(page)
  await page.keyboard.press('Control+Enter')
  const band = page.locator('#proofing-band')
  await expect(band.locator('[data-default]')).toHaveText('courrier')
  await expect(band).toBeFocused()
  await page.keyboard.press('ArrowLeft')
  await expect(band.getByRole('button', { name: 'courriers', exact: true })).toBeFocused()
  expect(await contents(page)).toEqual(before)
  await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap')).toHaveText('Ce courriers est prêt.')
  await expect(page.locator('.tiptap')).toBeFocused()
})

test('bandeaux : déplacement entre actions sans suppression et annulation au clavier', async ({
  page,
}) => {
  const before = await contents(page)
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  const field = page.getByLabel('Texte alternatif', { exact: true })
  await field.fill('Description provisoire')
  await field.evaluate((input: HTMLInputElement) => input.setSelectionRange(0, 0))
  await field.press('ArrowRight')
  await expect(field).toBeFocused()
  expect(await field.evaluate((input: HTMLInputElement) => input.selectionStart)).toBe(1)
  await page.locator('#cancel-properties').focus()
  await page.keyboard.press('ArrowRight')
  await expect(page.locator('#apply-properties')).toBeFocused()
  expect(await contents(page)).toEqual(before)
  await page.keyboard.press('Escape')
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.getByRole('button', { name: 'Supprimer · Image', exact: true }).click()
  await page.locator('#cancel-properties').focus()
  await page.keyboard.press('ArrowRight')
  await expect(page.locator('#apply-properties')).toBeFocused()
  expect(await contents(page)).toEqual(before)
  await page.keyboard.press('Escape')
  expect(await contents(page)).toEqual(before)
})

test('préférences : les champs natifs et les adresses multilignes conservent leurs touches', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  const size = page.getByLabel('Taille', { exact: true })
  await size.fill('18')
  await size.press('ArrowUp')
  await expect(size).toHaveValue('19')
  const address = page.locator('#side-content textarea').first()
  await address.fill('Première ligne')
  await address.press('End')
  await address.press('Enter')
  await address.press('x')
  await expect(address).toHaveValue('Première ligne\nx')
  await expect(page.locator('.side-panel')).toBeVisible()
  await address.press('Escape')
  await expect(page.locator('.side-panel')).toBeHidden()
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  await expect(size).not.toHaveValue('19')
  await expect(address).not.toHaveValue('Première ligne\nx')
})

test('recherche : le champ garde le focus et chaque flèche remplace la sélection précédente', async ({
  page,
}) => {
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent('<p>Bonjour, bonjour et BONJOUR.</p>')
    editor.commands.focus('start')
  })
  await page.getByRole('button', { name: 'Rechercher', exact: true }).click()
  const query = page.getByRole('combobox', { name: 'Rechercher', exact: true })
  await query.fill('bonjour')
  await expect(page.locator('.search-hit')).toHaveCount(3)
  await query.press('ArrowDown')
  await expect(query).toBeFocused()
  let range = await page.evaluate(() => ({
    from: window.__spike!.editor.state.selection.from,
    to: window.__spike!.editor.state.selection.to,
  }))
  expect(range).toEqual({ from: 1, to: 8 })
  await query.press('ArrowDown')
  await expect(query).toBeFocused()
  range = await page.evaluate(() => ({
    from: window.__spike!.editor.state.selection.from,
    to: window.__spike!.editor.state.selection.to,
  }))
  expect(range).toEqual({ from: 10, to: 17 })
  await query.press('ArrowUp')
  await expect(query).toBeFocused()
  await query.press('Tab')
  await expect(page.getByRole('textbox', { name: 'Remplacer par', exact: true })).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.locator('.side-panel')).toBeHidden()
})

test('dialogue de création : Tab atteint les actions et les flèches ne créent aucun document', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  const dialog = page.locator('dialog.app-dialog')
  await expect(dialog).toBeVisible()
  const model = dialog.getByRole('combobox', { name: 'Modèle', exact: true })
  await model.focus()
  await model.press('Tab')
  const cancel = dialog.getByRole('button', { name: 'Annuler', exact: true })
  await expect(cancel).toBeFocused()
  await page.keyboard.press('ArrowRight')
  await expect(dialog.getByRole('button', { name: 'Créer', exact: true })).toBeFocused()
  await expect(dialog).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(dialog).toBeHidden()
})

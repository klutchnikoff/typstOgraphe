import { expect, test, type Page } from '@playwright/test'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'

test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})
async function header(page: Page) {
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
  await expect(page.locator('#metadata-content')).toBeVisible()
}
test('trois onglets, sections décrites par le type, clavier et largeur adaptée', async ({
  page,
}) => {
  await expect(page.locator('#metadata-tabs button')).toHaveText([
    'Général',
    'Expéditeur',
    'En-tête',
  ])
  const general = page.getByRole('button', { name: 'Général', exact: true })
  await general.focus()
  await general.press('ArrowRight')
  await expect(page.getByRole('button', { name: 'Expéditeur', exact: true })).toBeFocused()
  await page.keyboard.press('ArrowRight')
  const tab = page.getByRole('button', { name: 'En-tête', exact: true })
  await expect(tab).toBeFocused()
  await expect(page.locator('#metadata-content legend')).toHaveText([
    'Destinataire',
    'Informations',
  ])
  await page.keyboard.press('Tab')
  const recipient = page.getByRole('textbox', { name: 'Destinataire', exact: true })
  await expect(recipient).toBeFocused()
  await page.keyboard.press('ControlOrMeta+A')
  await page.keyboard.insertText('Nouvelle destinataire')
  await expect(recipient).toHaveValue('Nouvelle destinataire')
  await expect(recipient).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.locator('#metadata-content')).toBeHidden()
  await expect(tab).toBeFocused()
  await tab.press('Enter')
  await page.getByLabel('Lieu', { exact: true }).fill('Lyon')
  await page.getByLabel('Date', { exact: true }).fill('2026-10-28')
  await page.getByRole('button', { name: 'Aujourd’hui', exact: true }).click()
  const today = await page.evaluate(() => {
    const now = new Date()
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
  })
  await expect(page.getByLabel('Date', { exact: true })).toHaveValue(today)
  for (const width of [1280, 700]) {
    await page.setViewportSize({ width, height: 700 })
    expect(
      await page
        .locator('#metadata-content')
        .evaluate((element) => element.scrollWidth - element.clientWidth),
    ).toBeLessThan(2)
    expect(
      await page.locator('#editor-scroll').evaluate((element) => element.clientHeight),
    ).toBeGreaterThan(100)
  }
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(recipient).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.locator('#metadata-content')).toBeHidden()
})

test('en-tête partagé, focus conservé, export Typst et sauvegarde sans duplication dans le corps', async ({
  page,
  context,
}) => {
  await header(page)
  const peer = await context.newPage()
  await peer.goto('/?client=peer-header')
  await peer.waitForFunction(() => window.__spike?.ready)
  await header(peer)
  const body = await page.evaluate(() => window.__spike!.editor.getJSON())
  await page.getByLabel('Destinataire', { exact: true }).fill('Bibliothèque universitaire')
  await page.getByLabel('Adresse', { exact: true }).fill('12 rue des Études\n69000 Lyon')
  await page.getByLabel('Objet', { exact: true }).fill('Demande de documentation')
  await page.getByLabel('Référence', { exact: true }).fill('DOC-42')
  await page.getByLabel('Lieu', { exact: true }).fill('Lyon')
  await page.getByLabel('Date', { exact: true }).fill('2026-10-28')
  await expect(peer.getByLabel('Destinataire', { exact: true })).toHaveValue(
    'Bibliothèque universitaire',
  )
  const recipient = page.getByLabel('Destinataire', { exact: true })
  await recipient.focus()
  await recipient.evaluate((input: HTMLInputElement) => input.setSelectionRange(3, 3))
  await peer.getByLabel('Objet', { exact: true }).fill('Objet relu ensemble')
  await expect(page.getByLabel('Objet', { exact: true })).toHaveValue('Objet relu ensemble')
  await expect(recipient).toBeFocused()
  expect(await recipient.evaluate((input: HTMLInputElement) => input.selectionStart)).toBe(3)
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(body)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
  const message = page.locator('#message')
  await expect(message).toContainText('PDF et sources Typst exportés', { timeout: 30000 })
  const folder = (await message.textContent())!.split(' · ').at(-1)!
  const source = await readFile(join(folder, 'main.typ'), 'utf8')
  expect(source).toContain('#show: courrier.with(')
  for (const fragment of [
    'destinataire: "Bibliothèque universitaire"',
    'objet: "Objet relu ensemble"',
    'reference: "DOC-42"',
    'lieu: "Lyon"',
    'date: "2026-10-28"',
  ])
    expect(source).toContain(fragment)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(message).toContainText('Session enregistrée', { timeout: 30000 })
  await peer.close()
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await header(page)
  await expect(page.getByLabel('Objet', { exact: true })).toHaveValue('Objet relu ensemble')
  await expect(page.getByLabel('Destinataire', { exact: true })).toHaveValue(
    'Bibliothèque universitaire',
  )
})

test('Général compact : choix utiles, valeurs imposées explicites et langue indépendante', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1600, height: 900 })
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  const panel = page.locator('#metadata-content')
  await expect(panel.getByRole('combobox')).toHaveCount(2)
  await expect(page.getByLabel('Format', { exact: true })).toHaveText('A4')
  await expect(page.getByLabel('Orientation', { exact: true })).toHaveText('Portrait')
  await expect(
    panel.getByText('Format et orientation définis par le modèle.', { exact: true }),
  ).toBeVisible()
  await expect(panel.locator('input[type=file]')).toHaveCount(0)
  await expect(panel.getByText('Puces des listes', { exact: true })).toHaveCount(0)
  const model = page.getByRole('combobox', { name: 'Modèle', exact: true })
  const language = page.getByRole('combobox', { name: 'Langue du document', exact: true })
  const geometry = await panel.evaluate((element) => {
    const label = element
      .querySelector('label[for="document-option-model"]')!
      .getBoundingClientRect()
    const select = element.querySelector('#document-option-model')!.getBoundingClientRect()
    const column = element.querySelector('.document-options-column')!.getBoundingClientRect()
    const language = element.querySelector('#document-option-language')!.getBoundingClientRect()
    return {
      sameLine: Math.abs(label.y + label.height / 2 - select.y - select.height / 2),
      separateColumns: language.x > column.right,
      controlWidth: select.width,
      columnWidth: column.width,
      height: element.getBoundingClientRect().height,
    }
  })
  expect(geometry.sameLine).toBeLessThan(2)
  expect(geometry.separateColumns).toBe(true)
  expect(geometry.controlWidth).toBeLessThan(geometry.columnWidth)
  expect(geometry.height).toBeLessThan(160)
  await model.focus()
  await page.keyboard.press('Tab')
  await expect(language).toBeFocused()
  await language.selectOption('en')
  await expect(language).toBeFocused()
  await expect(page.locator('.tiptap')).toHaveAttribute('lang', 'en')
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr')
  await page.setViewportSize({ width: 700, height: 700 })
  expect(await panel.evaluate((element) => element.scrollWidth - element.clientWidth)).toBeLessThan(
    2,
  )
  await page.keyboard.press('Escape')
  await expect(panel).toBeHidden()
})

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

test('Général compact : format et orientation libres, choix utiles et langue indépendante', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1600, height: 900 })
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  const panel = page.locator('#metadata-content')
  await expect(panel.getByRole('combobox')).toHaveCount(4)
  await expect(page.getByRole('combobox', { name: 'Format', exact: true })).toHaveValue('a4')
  await expect(page.getByRole('combobox', { name: 'Orientation', exact: true })).toHaveValue(
    'false',
  )
  await expect(panel.locator('output, #document-page-policy')).toHaveCount(0)
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
  await expect(page.getByRole('combobox', { name: 'Format', exact: true })).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(page.getByRole('combobox', { name: 'Orientation', exact: true })).toBeFocused()
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

test('tous les formats du catalogue sont réellement compilés en portrait et paysage, puis conservés', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  const paper = page.getByRole('combobox', { name: 'Format', exact: true })
  const orientation = page.getByRole('combobox', { name: 'Orientation', exact: true })
  const sizes: Record<string, [number, number]> = {
    a4: [210, 297],
    a5: [148, 210],
    'us-letter': [215.9, 279.4],
    'us-legal': [215.9, 355.6],
    a3: [297, 420],
  }
  expect(
    await paper
      .locator('option')
      .evaluateAll((options) => options.map((option) => (option as HTMLOptionElement).value)),
  ).toEqual(Object.keys(sizes))
  for (const [format, [width, height]] of Object.entries(sizes)) {
    await paper.selectOption(format)
    for (const landscape of ['false', 'true']) {
      await orientation.selectOption(landscape)
      const expected = landscape === 'true' ? height / width : width / height
      await expect
        .poll(() =>
          page
            .locator('#pages .preview-page svg')
            .first()
            .evaluate((svg) => {
              const box = (svg as SVGSVGElement).viewBox.baseVal
              return box.width / box.height
            }),
        )
        .toBeCloseTo(expected, 3)
      await expect(page.locator('#render-status')).toHaveText(/^\d+ pages? ·/)
    }
  }
  await paper.selectOption('a5')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(page.locator('#message')).toContainText('Session enregistrée', { timeout: 30000 })
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  await expect(page.getByRole('combobox', { name: 'Format', exact: true })).toHaveValue('a5')
  await expect(page.getByRole('combobox', { name: 'Orientation', exact: true })).toHaveValue('true')
})

test('expéditeur compact : identité, coordonnées et organisation, clavier et champs partagés', async ({
  page,
  context,
}) => {
  await page.setViewportSize({ width: 1600, height: 900 })
  await page.getByRole('button', { name: 'Expéditeur', exact: true }).click()
  const content = page.locator('#metadata-content')
  const labels = [
    'Nom complet',
    'Adresse',
    'Courriel',
    'Téléphone',
    'Organisme',
    'Service',
    'Fonction',
  ]
  await expect(content.locator('label')).toHaveText(labels)
  await expect(content.locator('legend')).toHaveText([
    'Coordonnées',
    'Organisation',
    'Signature du document',
  ])
  const name = content.getByLabel('Nom complet', { exact: true })
  await name.focus()
  for (const label of labels) {
    await expect(content.getByLabel(label, { exact: true })).toBeFocused()
    await page.keyboard.press('Tab')
  }
  await expect(content.getByLabel('Signature du document', { exact: true })).toBeFocused()
  await page.keyboard.press('Shift+Tab')
  await expect(content.getByLabel('Fonction', { exact: true })).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(content.getByLabel('Signature du document', { exact: true })).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(content.getByRole('button', { name: 'Sans signature', exact: true })).toBeFocused()
  const contact = content.locator('[data-section=contact]'),
    organisation = content.locator('[data-section=organisation]')
  const left = (await contact.boundingBox())!,
    right = (await organisation.boundingBox())!
  expect(Math.abs(left.y - right.y)).toBeLessThan(2)
  expect(right.x).toBeGreaterThan(left.x + left.width)
  for (const label of labels) {
    const field = content.getByLabel(label, { exact: true })
    const bounds = await field.boundingBox()
    const labelBounds = await content
      .locator('label')
      .filter({ hasText: new RegExp(`^${label}$`) })
      .boundingBox()
    expect(bounds!.x).toBeGreaterThan(labelBounds!.x + labelBounds!.width)
    expect(Math.abs(bounds!.y - labelBounds!.y)).toBeLessThan(12)
  }
  await name.fill('Camille Durand')
  await content.getByLabel('Adresse', { exact: true }).fill('12 rue des Lilas\n69000 Lyon')
  await content.getByLabel('Courriel', { exact: true }).fill('camille@example.org')
  await content.getByLabel('Téléphone', { exact: true }).fill('01 23 45 67 89')
  await content.getByLabel('Organisme', { exact: true }).fill('Université')
  await content.getByLabel('Service', { exact: true }).fill('Bibliothèque')
  await content.getByLabel('Fonction', { exact: true }).fill('Responsable')
  const peer = await context.newPage()
  await peer.goto('/?client=peer-sender')
  await peer.waitForFunction(() => window.__spike?.ready)
  await peer.getByRole('button', { name: 'Expéditeur', exact: true }).click()
  for (const label of labels)
    await expect(peer.locator('#metadata-content').getByLabel(label, { exact: true })).toHaveValue(
      await content.getByLabel(label, { exact: true }).inputValue(),
    )
  await name.focus()
  await name.evaluate((input: HTMLInputElement) => input.setSelectionRange(3, 3))
  await peer
    .locator('#metadata-content')
    .getByLabel('Service', { exact: true })
    .fill('Documentation')
  await expect(content.getByLabel('Service', { exact: true })).toHaveValue('Documentation')
  await expect(name).toBeFocused()
  expect(await name.evaluate((input: HTMLInputElement) => input.selectionStart)).toBe(3)
  for (const width of [1280, 700]) {
    await page.setViewportSize({ width, height: 700 })
    expect(
      await content.evaluate((element) => element.scrollWidth - element.clientWidth),
    ).toBeLessThan(2)
    expect(
      await page.locator('#editor-scroll').evaluate((element) => element.clientHeight),
    ).toBeGreaterThan(100)
  }
  expect((await organisation.boundingBox())!.y).toBeGreaterThan((await contact.boundingBox())!.y)
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(name).toBeVisible()
})

test('en-tête compact : destinataire, objet et référence puis lieu et date, sans débordement', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1600, height: 900 })
  await header(page)
  const content = page.locator('#metadata-content')
  const labels = ['Destinataire', 'Adresse', 'Objet', 'Référence', 'Lieu', 'Date']
  await expect(content.locator('label')).toHaveText(labels)
  const recipient = content.getByRole('group', { name: 'Destinataire', exact: true })
  const information = content.getByRole('group', { name: 'Informations', exact: true })
  await expect(recipient.locator('label')).toHaveText(labels.slice(0, 2))
  await expect(information.locator('label')).toHaveText(labels.slice(2))
  const recipientBox = (await recipient.boundingBox())!,
    informationBox = (await information.boundingBox())!
  expect(Math.abs(recipientBox.y - informationBox.y)).toBeLessThan(2)
  expect(informationBox.x).toBeGreaterThan(recipientBox.x + recipientBox.width)
  expect((await content.boundingBox())!.height).toBeLessThan(245)
  for (const label of labels) {
    const bounds = (await content.getByLabel(label, { exact: true }).boundingBox())!
    const labelBounds = (await content
      .locator('label')
      .filter({ hasText: new RegExp(`^${label}$`) })
      .boundingBox())!
    expect(bounds.x).toBeGreaterThan(labelBounds.x + labelBounds.width)
    expect(Math.abs(bounds.y - labelBounds.y)).toBeLessThan(12)
  }
  await content.getByLabel('Destinataire', { exact: true }).focus()
  // Date controls keep their native internal keyboard navigation.
  for (const label of labels) {
    await expect(content.getByLabel(label, { exact: true })).toBeFocused()
    if (label !== 'Date') await page.keyboard.press('Tab')
  }
  await content.getByRole('button', { name: 'Aujourd’hui', exact: true }).focus()
  await page.keyboard.press('Enter')
  const today = await page.evaluate(() => {
    const now = new Date()
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
  })
  await expect(content.getByLabel('Date', { exact: true })).toHaveValue(today)
  for (const width of [1280, 700]) {
    await page.setViewportSize({ width, height: 700 })
    expect(
      await content.evaluate((element) => element.scrollWidth - element.clientWidth),
    ).toBeLessThan(2)
    expect(
      (await content.getByLabel('Date', { exact: true }).boundingBox())!.width,
    ).toBeGreaterThan(120)
    expect(
      await page.locator('#editor-scroll').evaluate((element) => element.clientHeight),
    ).toBeGreaterThan(100)
  }
  expect((await information.boundingBox())!.y).toBeGreaterThan((await recipient.boundingBox())!.y)
  await page.keyboard.press('Escape')
  await expect(content).toBeHidden()
  await expect(page.getByRole('button', { name: 'En-tête', exact: true })).toBeFocused()
})

test('date facultative : suppression au clavier, partage, sauvegarde et export sans date', async ({
  page,
  context,
}) => {
  await header(page)
  const date = page.getByLabel('Date', { exact: true })
  const clear = page.getByRole('button', { name: 'Supprimer la date', exact: true })
  await date.fill('1875-01-19')
  await page.getByLabel('Lieu', { exact: true }).fill('')
  const peer = await context.newPage()
  await peer.goto('/?client=peer-date')
  await peer.waitForFunction(() => window.__spike?.ready)
  await header(peer)
  await expect(peer.getByLabel('Date', { exact: true })).toHaveValue('1875-01-19')
  await page.getByRole('button', { name: 'Aujourd’hui', exact: true }).focus()
  await page.keyboard.press('Tab')
  await expect(clear).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(date).toHaveValue('')
  await expect(date).toBeFocused()
  await expect(clear).toBeDisabled()
  await expect(peer.getByLabel('Date', { exact: true })).toHaveValue('')
  await expect(peer.getByRole('button', { name: 'Supprimer la date', exact: true })).toBeDisabled()
  await expect(page.locator('#render-status')).toHaveText(/^\d+ pages? ·/)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
  const message = page.locator('#message')
  await expect(message).toContainText('PDF et sources Typst exportés', { timeout: 30000 })
  const folder = (await message.textContent())!.split(' · ').at(-1)!
  const source = await readFile(join(folder, 'main.typ'), 'utf8')
  expect(source).toContain('date: ""')
  expect(source).toContain('lieu: ""')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(message).toContainText('Session enregistrée', { timeout: 30000 })
  await peer.close()
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await header(page)
  await expect(page.getByLabel('Date', { exact: true })).toHaveValue('')
  await expect(page.getByRole('button', { name: 'Supprimer la date', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: 'Aujourd’hui', exact: true }).click()
  await expect(page.getByLabel('Date', { exact: true })).not.toHaveValue('')
  await expect(page.getByRole('button', { name: 'Supprimer la date', exact: true })).toBeEnabled()
})

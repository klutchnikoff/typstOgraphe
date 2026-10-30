import { confirmNewDocument } from './document-lifecycle'
import { expect, test, type Page } from '@playwright/test'

const dialog = (page: Page) => page.locator('dialog.form-dialog')
const tab = (page: Page, name: string) => page.locator(`#metadata-tab-${name}`).click()
const records = (page: Page, kind: string) =>
  page.locator(`[data-section="${kind}"] .metadata-record`)
const add = (page: Page, kind: string) =>
  page
    .getByRole('button', { name: `Ajouter · ${kind}`, exact: true })
    .last()
    .click()
const confirm = async (page: Page) => {
  await dialog(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(() => window.__spike!.provider.flush())
}
const name = (page: Page) => dialog(page).getByLabel('Nom complet', { exact: true })
const data = (page: Page) =>
  page.evaluate(() =>
    (window.__spike!.provider.doc.getMap('settings').get('collections') as any).toJSON(),
  )
async function ready(page: Page, id = 'main') {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
async function create(page: Page) {
  await ready(page)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await confirmNewDocument(page)
  await tab(page, 'affiliations')
  await add(page, 'Affiliation')
  await dialog(page)
    .getByLabel('Signature officielle', { exact: true })
    .fill('Univ Rennes, CNRS, IRMAR')
  await confirm(page)
  await tab(page, 'authors')
}
async function author(page: Page) {
  await add(page, 'Auteur')
  await name(page).fill('Camille Martin')
  await dialog(page).getByLabel('Univ Rennes, CNRS, IRMAR', { exact: true }).check()
  await confirm(page)
}
test.beforeEach(async ({ request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
})

test('dialogue réutilisable : clavier, fond flouté, brouillon, annulation et validation atomique', async ({
  page,
}) => {
  await create(page)
  await author(page)
  const before = await data(page)
  const trigger = records(page, 'authors').getByRole('button', {
    name: 'Modifier · Auteur',
    exact: true,
  })
  await trigger.click()
  await expect(name(page)).toBeFocused()
  await name(page).fill('Brouillon à annuler')
  await expect.poll(() => data(page)).toEqual(before)
  // The background is inert and does not dismiss a draft on an incidental click.
  await page.mouse.click(8, 8)
  await expect(dialog(page)).toBeVisible()
  await name(page).focus()
  await page.keyboard.press('Shift+Tab')
  await expect(
    dialog(page).getByRole('button', { name: 'Annuler', exact: true }).first(),
  ).toBeFocused()
  await page.keyboard.press('Shift+Tab')
  await expect(dialog(page).getByRole('button', { name: 'Valider', exact: true })).toBeFocused()
  await page.keyboard.press('ArrowLeft')
  await expect(
    dialog(page).getByRole('button', { name: 'Annuler', exact: true }).last(),
  ).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(dialog(page)).toHaveCount(0)
  await expect(trigger).toBeFocused()
  await expect.poll(() => data(page)).toEqual(before)

  await trigger.click()
  await name(page).fill('Camille Durand')
  await dialog(page).getByLabel('Courriel', { exact: true }).fill('camille@example.org')
  await dialog(page).getByLabel('Auteur correspondant', { exact: true }).check()
  // A composing Enter, including the modified form, must not publish.
  await name(page).dispatchEvent('keydown', { key: 'Enter', ctrlKey: true, isComposing: true })
  await expect(dialog(page)).toBeVisible()
  await expect.poll(() => data(page)).toEqual(before)
  await name(page).press('ControlOrMeta+Enter')
  await expect(dialog(page)).toHaveCount(0)
  await expect(trigger).toBeFocused()
  await expect(records(page, 'authors')).toContainText('camille@example.org')
  const saved = await data(page)
  expect(Object.keys(saved.authors)).toEqual(Object.keys(before.authors))
  const body = page.getByRole('textbox', { name: 'Corps du document', exact: true })
  await body.focus()
  await body.press('ControlOrMeta+z')
  await expect.poll(() => data(page)).toEqual(before)
  await body.press('ControlOrMeta+Shift+z')
  await expect.poll(() => data(page)).toEqual(saved)

  // Responsive geometry and actual backdrop style, without screenshots.
  await trigger.click()
  for (const width of [420, 920, 1280]) {
    await page.setViewportSize({ width, height: 740 })
    for (const colorScheme of ['light', 'dark'] as const) {
      await page.emulateMedia({ colorScheme })
      const layout = await dialog(page).evaluate((element) => {
        const rect = element.getBoundingClientRect()
        const backdrop = getComputedStyle(element, '::backdrop')
        return {
          fits:
            rect.left >= 0 &&
            rect.right <= innerWidth &&
            rect.top >= 0 &&
            rect.bottom <= innerHeight,
          centered: Math.abs((rect.left + rect.right) / 2 - innerWidth / 2) < 1,
          overflow: element.scrollWidth > element.clientWidth,
          blur: backdrop.backdropFilter || backdrop.getPropertyValue('-webkit-backdrop-filter'),
        }
      })
      expect(layout).toEqual({ fits: true, centered: true, overflow: false, blur: 'blur(3px)' })
    }
  }
  await page.keyboard.press('Escape')
  await tab(page, 'affiliations')
  await records(page, 'affiliations').locator('summary').click()
  const address = dialog(page).getByLabel('Signature officielle', { exact: true })
  await expect(address).toBeFocused()
  await address.press('End')
  await address.press('Enter')
  await page.keyboard.insertText('F-35000 Rennes, France')
  await expect(dialog(page)).toBeVisible()
  await address.press('ControlOrMeta+Enter')
  await expect(records(page, 'affiliations')).toContainText('F-35000 Rennes, France')
})

test('une modification concurrente du même auteur est signalée et conserve le brouillon', async ({
  page,
  context,
}) => {
  await create(page)
  await author(page)
  const peer = await context.newPage()
  await ready(peer, `record-edit-peer-${Date.now()}`)
  await tab(peer, 'authors')
  await records(page, 'authors').locator('summary').click()
  await name(page).fill('Ma proposition privée')
  await records(peer, 'authors').locator('summary').click()
  await expect(name(peer)).toHaveValue('Camille Martin')
  await name(peer).fill('Modification de la collègue')
  await confirm(peer)
  await expect(records(page, 'authors')).toContainText('Modification de la collègue')
  await confirm(page)
  await expect(dialog(page).getByRole('alert')).toContainText('modifié ou supprimé ailleurs')
  await expect(name(page)).toHaveValue('Ma proposition privée')
  await expect(records(peer, 'authors')).toContainText('Modification de la collègue')
  await page.keyboard.press('Escape')
  await expect(records(page, 'authors')).toContainText('Modification de la collègue')
  await peer.close()
})

test('une affiliation supprimée reste visible dans le brouillon et exige le retrait explicite de son association', async ({
  page,
  context,
}) => {
  await create(page)
  const peer = await context.newPage()
  await ready(peer, `record-reference-peer-${Date.now()}`)
  await tab(peer, 'affiliations')
  await add(page, 'Auteur')
  await name(page).fill('Camille')
  await dialog(page).getByLabel('Univ Rennes, CNRS, IRMAR', { exact: true }).check()
  await records(peer, 'affiliations')
    .getByRole('button', { name: 'Supprimer · Affiliation', exact: true })
    .click()
  await records(peer, 'affiliations')
    .getByRole('button', { name: 'Supprimer', exact: true })
    .click()
  const missing = dialog(page).getByRole('checkbox', {
    name: 'Affiliation · Élément supprimé',
    exact: true,
  })
  await expect(missing).toBeChecked()
  await confirm(page)
  await expect(dialog(page).getByRole('alert')).toContainText('sélectionnée a été supprimée')
  await expect(records(peer, 'authors')).toHaveCount(0)
  await missing.uncheck()
  await confirm(page)
  await expect(records(page, 'authors')).toHaveCount(1)
  const snapshot = await data(page)
  expect(Object.values(snapshot.authors)[0]).toMatchObject({
    name: 'Camille',
    'affiliation-ids': [],
  })
  await peer.close()
})

import { expect, test, type Page } from '@playwright/test'

const official =
  'Univ Rennes, CNRS, IRMAR (Institut de Recherche Mathématique de Rennes) - UMR 6625, F-35000 Rennes, France'
const section = (page: Page, parameter: string) => page.locator(`[data-section="${parameter}"]`)
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
const draft = (page: Page) => page.locator('.metadata-record-draft')
async function tab(page: Page, id: string) {
  await page.locator(`#metadata-tab-${id}`).click()
}
async function ready(page: Page, id = 'main') {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready, undefined, { timeout: 15_000 })
}
async function create(page: Page) {
  await ready(page)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await expect(page.locator('dialog select').nth(1)).toHaveValue('article.model.working')
  await page.getByRole('button', { name: 'Créer', exact: true }).click()
  await tab(page, 'titles')
}
async function add(page: Page, label: string, index = -1) {
  const buttons = page.getByRole('button', { name: `Ajouter · ${label}`, exact: true })
  await (index < 0 ? buttons.last() : buttons.nth(index)).click()
  await expect(draft(page)).toBeVisible()
}
async function confirm(page: Page) {
  await draft(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(draft(page)).toHaveCount(0)
}
test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})

test.afterEach(async ({ page }, info) => {
  if (info.status !== info.expectedStatus)
    console.error(
      'État de synchronisation :',
      await page
        .evaluate(() => ({
          rejected: window.__spike?.provider.rejected,
          history: window.__spike?.provider.historyState,
        }))
        .catch(() => null),
    )
})

test('Article : onglets, auteurs partagés, langues, annulation et fichier autonome', async ({
  page,
  context,
}) => {
  test.setTimeout(90_000)
  page.setDefaultTimeout(15_000)
  const errors: string[] = []
  page.on('pageerror', (error) => {
    errors.push(error.message)
    console.error(error.stack)
  })
  await create(page)
  const title = page.getByRole('textbox', { name: 'Titre', exact: true })
  await title.fill('Travail universitaire et institutions')
  await expect(title).toBeFocused()
  await page
    .getByRole('textbox', { name: 'Titre court', exact: true })
    .fill('Travail universitaire')
  await expect(page.getByRole('textbox', { name: 'Résumé', exact: true })).toHaveCount(0)
  await tab(page, 'affiliations')
  const affiliations = section(page, 'affiliations')
  await add(page, 'Affiliation')
  await draft(page).getByLabel('Signature officielle', { exact: true }).fill(official)
  await confirm(page)
  await tab(page, 'authors')
  const authors = section(page, 'authors')
  await add(page, 'Auteur')
  await draft(page).getByLabel('Nom complet', { exact: true }).fill('Camille Martin')
  await draft(page).getByLabel(official, { exact: true }).check()
  await draft(page).getByLabel('Auteur correspondant', { exact: true }).check()
  await draft(page).getByLabel('Courriel', { exact: true }).fill('camille@example.org')
  await confirm(page)
  await add(page, 'Auteur')
  await draft(page).getByLabel('Nom complet', { exact: true }).fill('Camille Martin')
  await draft(page).getByLabel(official, { exact: true }).check()
  await confirm(page)
  await authors
    .locator('details')
    .nth(1)
    .getByRole('button', { name: 'Monter · Auteur', exact: true })
    .click()
  await page.evaluate(() => window.__spike!.provider.flush())
  await authors.locator('summary').first().click()
  await expect(draft(page).getByLabel('Auteur correspondant')).not.toBeChecked()
  await page.keyboard.press('Escape')
  await tab(page, 'abstracts')
  const locales = section(page, 'localizations')
  await locales
    .getByRole('textbox', { name: 'Résumé', exact: true })
    .fill('Un résumé précis et lisible.')
  await locales
    .getByLabel('Mots-clés', { exact: true })
    .fill('enseignement supérieur\norganisation du travail')
  await tab(page, 'titles')
  await add(page, 'Ensemble linguistique')
  await expect(draft(page).getByLabel('Langue', { exact: true })).toHaveValue('en')
  await draft(page)
    .getByRole('textbox', { name: 'Titre', exact: true })
    .fill('Academic work and institutions')
  await draft(page).getByRole('textbox', { name: 'Titre court', exact: true }).fill('Academic work')
  await confirm(page)
  await tab(page, 'abstracts')
  const english = locales.locator('details').nth(1)
  await english.locator('summary').click()
  await english
    .getByRole('textbox', { name: 'Résumé', exact: true })
    .fill('A concise English abstract.')
  await english.getByLabel('Mots-clés', { exact: true }).fill('higher education\nacademic work')
  // Rich-zone undo uses the document history, and survives closing the tab.
  const abstract = english.getByRole('textbox', { name: 'Résumé', exact: true })
  await abstract.press('ControlOrMeta+a')
  await expect
    .poll(() => page.evaluate(() => window.getSelection()?.toString().trim()))
    .toBe('A concise English abstract.')
  await english.getByRole('button', { name: 'Italique · Résumé', exact: true }).click()
  await expect(abstract.locator('em')).not.toHaveCount(0)
  await abstract.press('ControlOrMeta+z')
  await expect(abstract.locator('em')).toHaveCount(0)
  await expect(abstract).toBeFocused()
  await abstract.press('ControlOrMeta+Shift+z')
  await expect(abstract.locator('em')).not.toHaveCount(0)
  await abstract.press('ControlOrMeta+z')
  await expect(abstract.locator('em')).toHaveCount(0)
  await abstract.press('ControlOrMeta+k')
  const link = page.locator('#metadata-content .context-band:not([hidden])')
  await link.locator('[id$=parameter-url]').fill('https://example.org/recherche')
  await link.getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(abstract.locator('a')).toHaveAttribute('href', 'https://example.org/recherche')
  await expect(abstract).toHaveText('A concise English abstract.')
  await expect(abstract).toBeFocused()
  await tab(page, 'titles')
  await expect(title).toHaveText('Travail universitaire et institutions')
  // Removing a used affiliation is explicit and undo restores its associations.
  await tab(page, 'affiliations')
  await affiliations.getByRole('button', { name: 'Supprimer · Affiliation', exact: true }).click()
  const removal = affiliations.locator('.properties')
  await removal.getByRole('button', { name: 'Supprimer', exact: true }).click()
  await expect(removal).toContainText('Cette affiliation est utilisée')
  await removal.getByRole('checkbox').check()
  await removal.getByRole('button', { name: 'Supprimer', exact: true }).click()
  await expect(affiliations.locator('details')).toHaveCount(0)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(affiliations.locator('details')).toHaveCount(1)
  await tab(page, 'authors')
  await authors.locator('summary').first().click()
  await expect(draft(page).getByLabel(official, { exact: true }).first()).toBeChecked()
  await page.keyboard.press('Escape')
  await tab(page, 'general')
  await page.getByLabel('Langue du document', { exact: true }).selectOption('en')
  await tab(page, 'titles')
  await expect(title).toHaveText('Academic work and institutions')
  // Independent peer changes refresh references without disturbing the active author field.
  const peer = await context.newPage()
  await ready(peer, `article-peer-${Date.now()}`)
  await tab(peer, 'affiliations')
  await tab(page, 'authors')
  await authors
    .locator('details')
    .first()
    .getByRole('button', { name: 'Modifier · Auteur', exact: true })
    .click()
  const name = draft(page).getByLabel('Nom complet', { exact: true })
  await name.focus()
  await section(peer, 'affiliations').locator('details summary').click()
  await draft(peer)
    .getByLabel('Signature officielle', { exact: true })
    .fill(official + ' (signature officielle)')
  await confirm(peer)
  await page.bringToFront()
  await expect(name).toBeFocused()
  await expect(
    draft(page)
      .getByLabel(official + ' (signature officielle)', { exact: true })
      .first(),
  ).toBeChecked()
  await page.keyboard.press('Escape')
  await peer.close()
  await body(page).fill('Le corps de cet article reste indépendant de son en-tête.')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(page.locator('#message')).toContainText('Session enregistrée', { timeout: 30_000 })
  expect(errors).toEqual([])
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready, undefined, { timeout: 15_000 })
  errors.length = 0
  await expect(body(page)).toContainText('Le corps de cet article')
  await tab(page, 'titles')
  await expect(title).toHaveText('Academic work and institutions')
  await tab(page, 'authors')
  await expect(authors.locator('details')).toHaveCount(2)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
  await expect(page.locator('#message')).toContainText('sources Typst exportés', {
    timeout: 30_000,
  })
  await page.setViewportSize({ width: 920, height: 740 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  )
  expect(errors).toEqual([])
})

test('les zones riches partagent les raccourcis Typst et la sémantique des retours du corps', async ({
  page,
}) => {
  await create(page)
  const title = page.getByRole('textbox', { name: 'Titre', exact: true })
  await title.fill('Un titre')
  await title.press('End')
  await title.press('Enter')
  await expect(title.locator('p')).toHaveCount(1)
  await expect(title).toHaveText('Un titre')
  await tab(page, 'abstracts')
  const abstract = section(page, 'localizations').getByRole('textbox', {
    name: 'Résumé',
    exact: true,
  })
  await abstract.fill('Première ligne')
  await abstract.press('End')
  await abstract.press('Enter')
  await page.keyboard.type('Deuxième ligne')
  await expect(abstract.locator('p')).toHaveCount(1)
  await expect(abstract.locator('br[data-soft-break]')).toHaveCount(1)
  await abstract.press('Enter')
  await abstract.press('Enter')
  await page.keyboard.type('*important*')
  await expect(abstract.locator('p')).toHaveCount(2)
  await expect(abstract.locator('strong')).toHaveText('important')
  await expect(abstract.locator('.paragraph-marker')).toHaveCount(1)
  await abstract.press('Enter')
  await abstract.press('Enter')
  await abstract.press('Enter')
  await page.keyboard.type('- ')
  await expect(abstract.locator('p')).toHaveCount(3)
  await expect(abstract.locator('ul')).toHaveCount(0)
  await expect(abstract).toContainText('- ')
  await tab(page, 'titles')
  await title.click()
  await title.press('Home')
  await title.press('ControlOrMeta+b')
  await page.keyboard.type('Bref ')
  await expect(title.locator('strong')).toHaveText('Bref ')
  await expect(body(page)).toBeEmpty()
})

test('les ajouts par + restent provisoires et Échap restaure le focus sans annuler le travail', async ({
  page,
  context,
}) => {
  await create(page)
  await expect(page.locator('#metadata-tabs button')).toHaveText([
    'Général',
    'Titres',
    'Affiliations',
    'Auteurs',
    'Résumés',
    'Bibliographie',
  ])
  await tab(page, 'authors')
  const authors = section(page, 'authors')
  await add(page, 'Auteur')
  const input = draft(page).getByLabel('Nom complet', { exact: true })
  await expect(input).toBeFocused()
  await input.fill('Premier auteur')
  await input.press('Enter')
  await expect(draft(page)).toHaveCount(0)
  await expect(authors.locator('summary').first()).toContainText('Premier auteur')
  const plus = authors.getByRole('button', { name: 'Ajouter · Auteur', exact: true })
  await expect(plus).toBeFocused()
  expect(await plus.textContent()).toBe('')
  const box = await plus.boundingBox()
  const row = await authors.locator('summary').first().boundingBox()
  expect(box!.x + box!.width).toBeCloseTo(row!.x + row!.width, 0)
  // A draft is absent from the shared document and does not erase a peer's intervening edit.
  const peer = await context.newPage()
  await ready(peer, `article-draft-peer-${Date.now()}`)
  await tab(peer, 'authors')
  await add(page, 'Auteur')
  await input.fill('Auteur provisoire')
  await expect(section(peer, 'authors').locator('details')).toHaveCount(1)
  await section(peer, 'authors')
    .getByRole('button', { name: 'Modifier · Auteur', exact: true })
    .click()
  await draft(peer)
    .getByLabel('Nom complet', { exact: true })
    .fill('Auteur modifié par une collègue')
  await confirm(peer)
  await page.bringToFront()
  await expect(input).toBeFocused()
  await input.press('Escape')
  await expect(draft(page)).toHaveCount(0)
  await expect(page.locator('#metadata-content')).toBeVisible()
  await expect(plus).toBeFocused()
  await expect(authors.locator('summary')).toContainText('Auteur modifié par une collègue')
  // Insert after the selected row, not necessarily at the end.
  await add(page, 'Auteur')
  await input.fill('Dernier auteur')
  await confirm(page)
  await add(page, 'Auteur', 0)
  await input.fill('Auteur intermédiaire')
  await confirm(page)
  await expect(authors.locator('.metadata-record-caption')).toHaveText([
    'Auteur modifié par une collègue',
    'Auteur intermédiaire',
    'Dernier auteur',
  ])
  await peer.close()
  await tab(page, 'affiliations')
  await add(page, 'Affiliation')
  const address = draft(page).getByLabel('Signature officielle', { exact: true })
  await expect(address).toBeFocused()
  await address.fill('Affiliation à annuler')
  await address.press('Escape')
  await expect(draft(page)).toHaveCount(0)
  await expect(section(page, 'affiliations').locator('details')).toHaveCount(0)
  await expect(
    page.getByRole('button', { name: 'Ajouter · Affiliation', exact: true }),
  ).toBeFocused()
  await add(page, 'Affiliation')
  await address.fill(official)
  await confirm(page)
  await add(page, 'Affiliation')
  await address.fill('Une seconde affiliation')
  await address.press('Escape')
  await expect(section(page, 'affiliations').locator('details')).toHaveCount(1)
  await expect(section(page, 'affiliations').locator('summary')).toContainText(official)
  // Keyboard navigation between tabs keeps the shared horizontal navigation contract.
  await page.locator('#metadata-tab-affiliations').focus()
  await page.keyboard.press('ArrowRight')
  await expect(page.locator('#metadata-tab-authors')).toBeFocused()
  await expect(authors.locator('details')).toHaveCount(3)
  await page.setViewportSize({ width: 920, height: 740 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  )
})

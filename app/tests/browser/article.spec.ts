import { expect, test, type Page } from '@playwright/test'

const official =
  'Univ Rennes, CNRS, IRMAR (Institut de Recherche Mathématique de Rennes) - UMR 6625, F-35000 Rennes, France'
const section = (page: Page, parameter: string) => page.locator(`[data-section="${parameter}"]`)
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
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
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
}
test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})

test('Article : champs riches, auteurs partagés, langues, annulation et fichier autonome', async ({
  page,
  context,
}) => {
  test.setTimeout(90_000)
  page.setDefaultTimeout(15_000)
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await create(page)
  const title = page.getByRole('textbox', { name: 'Titre', exact: true })
  await title.fill('Travail universitaire et institutions')
  await expect(title).toBeFocused()
  await page
    .getByRole('textbox', { name: 'Titre court', exact: true })
    .fill('Travail universitaire')
  const affiliations = section(page, 'affiliations')
  await affiliations.getByRole('button', { name: 'Ajouter · Affiliation', exact: true }).click()
  await affiliations.getByLabel('Signature officielle', { exact: true }).fill(official)
  const authors = section(page, 'authors')
  await authors.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).click()
  await authors.getByLabel('Nom complet', { exact: true }).fill('Camille Martin')
  await authors.getByLabel(official, { exact: true }).check()
  await authors.getByLabel('Auteur correspondant', { exact: true }).check()
  await authors.getByLabel('Courriel', { exact: true }).fill('camille@example.org')
  await authors.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).click()
  const second = authors.locator('details').nth(1)
  await second.getByLabel('Nom complet', { exact: true }).fill('Camille Martin')
  await second.getByLabel(official, { exact: true }).check()
  await second.getByRole('button', { name: 'Monter · Auteur', exact: true }).click()
  await expect(
    authors.locator('details').first().getByLabel('Auteur correspondant'),
  ).not.toBeChecked()
  const locales = section(page, 'localizations')
  await locales
    .getByRole('textbox', { name: 'Résumé', exact: true })
    .fill('Un résumé précis et lisible.')
  await locales
    .getByLabel('Mots-clés', { exact: true })
    .fill('enseignement supérieur\norganisation du travail')
  await locales
    .getByRole('button', { name: 'Ajouter · Ensemble linguistique', exact: true })
    .click()
  const english = locales.locator('details').nth(1)
  await expect(english.getByLabel('Langue', { exact: true })).toHaveValue('en')
  await english
    .getByRole('textbox', { name: 'Titre', exact: true })
    .fill('Academic work and institutions')
  await english.getByRole('textbox', { name: 'Titre court', exact: true }).fill('Academic work')
  await english
    .getByRole('textbox', { name: 'Résumé', exact: true })
    .fill('A concise English abstract.')
  await english.getByLabel('Mots-clés', { exact: true }).fill('higher education\nacademic work')
  // Rich-zone undo uses the document history, and survives closing the header.
  const abstract = english.getByRole('textbox', { name: 'Résumé', exact: true })
  await abstract.press('End')
  await abstract.press('ControlOrMeta+Shift+ArrowLeft')
  await english.getByRole('button', { name: 'Italique · Résumé', exact: true }).click()
  await expect(abstract.locator('em')).not.toHaveCount(0)
  await abstract.press('ControlOrMeta+z')
  await expect(abstract.locator('em')).toHaveCount(0)
  await expect(abstract).toBeFocused()
  await abstract.press('ControlOrMeta+Shift+z')
  await expect(abstract.locator('em')).not.toHaveCount(0)
  await abstract.press('ControlOrMeta+z')
  await expect(abstract.locator('em')).toHaveCount(0)
  // The shared link band edits this rich zone, preserving its text and selection.
  await abstract.press('ControlOrMeta+k')
  const link = page.locator('#metadata-content .context-band:not([hidden])')
  await link.locator('[id$=parameter-url]').fill('https://example.org/recherche')
  await link.getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(abstract.locator('a')).toHaveAttribute('href', 'https://example.org/recherche')
  await expect(abstract).toHaveText('A concise English abstract.')
  await expect(abstract).toBeFocused()
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
  await expect(page.getByRole('textbox', { name: 'Titre', exact: true }).first()).toHaveText(
    'Travail universitaire et institutions',
  )
  // Removing a used affiliation is explicit and undo restores its associations.
  await affiliations.getByRole('button', { name: 'Supprimer · Affiliation', exact: true }).click()
  const removal = affiliations.locator('.properties')
  await removal.getByRole('button', { name: 'Supprimer', exact: true }).click()
  await expect(removal).toContainText('Cette affiliation est utilisée')
  await removal.getByRole('checkbox').check()
  await removal.getByRole('button', { name: 'Supprimer', exact: true }).click()
  await expect(affiliations.locator('details')).toHaveCount(0)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(affiliations.locator('details')).toHaveCount(1)
  await expect(authors.getByLabel(official, { exact: true }).first()).toBeChecked()
  // Different language, same preserved records, no automatic translation.
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  await page.getByLabel('Langue du document', { exact: true }).selectOption('en')
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
  await expect(
    page.locator('.metadata-primary-fields').getByRole('textbox', { name: 'Titre', exact: true }),
  ).toHaveText('Academic work and institutions')
  // A peer edits an independent field while the current input retains focus.
  const peer = await context.newPage()
  await ready(peer, `article-peer-${Date.now()}`)
  await peer.getByRole('button', { name: 'En-tête', exact: true }).click()
  await authors
    .locator('details')
    .first()
    .getByRole('button', { name: 'Modifier · Auteur', exact: true })
    .click()
  await authors.locator('details').first().getByLabel('Nom complet', { exact: true }).focus()
  await section(peer, 'affiliations').locator('details summary').click()
  await section(peer, 'affiliations')
    .getByLabel('Signature officielle', { exact: true })
    .fill(official + ' (signature officielle)')
  await page.bringToFront()
  await expect(
    authors.locator('details').first().getByLabel('Nom complet', { exact: true }),
  ).toBeFocused()
  await expect(affiliations.locator('summary').first()).toContainText('signature officielle')
  await peer.close()
  await body(page).fill('Le corps de cet article reste indépendant de son en-tête.')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(page.locator('#message')).toContainText('Session enregistrée', { timeout: 30_000 })
  expect(errors).toEqual([])
  // Navigation aborts diagnostic fetches in WebKit; check runtime errors on each live page.
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready, undefined, { timeout: 15_000 })
  errors.length = 0
  await expect(body(page)).toContainText('Le corps de cet article')
  await page.getByRole('button', { name: 'En-tête', exact: true }).click()
  await expect(
    page.locator('.metadata-primary-fields').getByRole('textbox', { name: 'Titre', exact: true }),
  ).toHaveText('Academic work and institutions')
  await expect(section(page, 'authors').locator('details')).toHaveCount(2)
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
  await title.click()
  await title.press('Home')
  await title.press('ControlOrMeta+b')
  await page.keyboard.type('Bref ')
  await expect(title.locator('strong')).toHaveText('Bref ')
  await expect(body(page)).toBeEmpty()
})

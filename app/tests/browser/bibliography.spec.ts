import { confirmNewDocument } from './document-lifecycle'
import { expect, test, type Page } from '@playwright/test'
const bib =
  '@book{bourdieu1980, author={Bourdieu, Pierre}, title={Le sens pratique}, year={1980}, publisher={Minuit}}'
const yaml =
  'durand2020:\n  type: Article\n  title: Le travail collectif\n  author: Durand, Camille\n  date: 2020\n  parent:\n    type: Periodical\n    title: Revue exemple\n'
const tab = (page: Page, id: string) => page.locator(`#metadata-tab-${id}`).click()
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
const band = (page: Page) => page.locator('#citation-properties')
const files = (page: Page) => page.getByLabel('Fichiers bibliographiques', { exact: true })
const file = (name: string, contents: string) => ({
  name,
  mimeType: 'text/plain',
  buffer: Buffer.from(contents),
})
const error = (page: Page) => page.locator('.bibliography-settings > [role=alert]')
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
  await tab(page, 'titles')
  await page.getByRole('textbox', { name: 'Titre', exact: true }).fill('Article bibliographique')
  await tab(page, 'bibliography')
  await files(page).setInputFiles(file('livres.bib', bib))
  await expect(page.locator('.bibliography-file-row')).toHaveCount(1)
  await files(page).setInputFiles(file('articles.yaml', yaml))
  await expect(page.locator('.bibliography-file-row')).toHaveCount(2)
  await expect(error(page)).toBeEmpty()
}
async function choose(page: Page, query: string) {
  const search = band(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill(query)
  await expect(band(page).locator('[role=option]:not(:disabled)')).toHaveCount(1)
  await search.press('ArrowDown')
  await expect(search).toBeFocused()
  await search.press('Enter')
}
async function insert(page: Page) {
  await body(page).fill('Une lecture du travail. ')
  await page.getByRole('button', { name: 'Citer une référence', exact: true }).click()
  await expect(band(page).getByRole('combobox', { name: 'Rechercher une référence' })).toBeFocused()
  await choose(page, 'sens bordieu')
  await band(page).getByLabel('Pages ou chapitre · bourdieu1980', { exact: true }).fill('p. 23–25')
  await choose(page, 'travail DURAND')
  await band(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(band(page)).toBeHidden()
  await expect(body(page).locator('.citation-ref')).toHaveCount(1)
  await expect(body(page).locator('.citation-ref')).toContainText('Bourdieu, 1980')
  await expect(body(page).locator('.citation-ref')).toContainText('Durand, 2020')
  await expect(body(page)).toBeFocused()
}
test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})

test('bibliothèque multi-format, recherche fuzzy, badge, clavier et rendu natif', async ({
  page,
}) => {
  await create(page)
  await insert(page)
  await expect(body(page).locator('.citation-ref')).toHaveClass(/inline-object-active/)
  await body(page).press('ControlOrMeta+Enter')
  await expect(band(page).getByRole('combobox', { name: 'Rechercher une référence' })).toBeFocused()
  await band(page).getByLabel('Forme de la citation', { exact: true }).selectOption('prose')
  await band(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(band(page)).toBeHidden()
  const citation = await page.evaluate(() => {
    let found: any
    window.__spike!.editor.state.doc.descendants((node) => {
      if (node.type.name === 'citation') found = node.attrs
    })
    return found
  })
  expect(citation.form).toBe('prose')
  expect(citation.items).toHaveLength(2)
  expect(citation.items[0].supplement).toBe('p. 23–25')
  await expect(page.locator('#render-status')).toContainText('page')
  await expect(page.locator('#render-status')).not.toHaveClass(/error/)
  // Cancellation and deletion use the same protected inline object path as footnotes.
  await body(page).press('Backspace')
  await expect(page.locator('#property-title')).toHaveText('Supprimer la citation')
  await page.locator('#cancel-properties').click()
  await expect(body(page).locator('.citation-ref')).toHaveCount(1)
  await body(page).press('Backspace')
  await page.locator('#apply-properties').click()
  await expect(body(page).locator('.citation-ref')).toHaveCount(0)
  await expect(page.locator('#properties')).toBeHidden()
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(body(page).locator('.citation-ref')).toHaveCount(1)
  await tab(page, 'general')
  await expect(page.getByLabel('Afficher le sommaire', { exact: true })).not.toBeChecked()
  await page.getByLabel('Afficher le sommaire', { exact: true }).check()
  await tab(page, 'bibliography')
  await page.getByLabel('Style bibliographique', { exact: true }).selectOption('ieee')
  await expect(body(page).locator('.citation-ref')).toContainText('Bourdieu, 1980')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(page.locator('#message')).toContainText('Session enregistrée')
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(body(page).locator('.citation-ref')).toContainText('Bourdieu, 1980')
  await tab(page, 'general')
  await expect(page.getByLabel('Afficher le sommaire', { exact: true })).toBeChecked()
  await tab(page, 'bibliography')
  await expect(page.locator('.bibliography-file-row')).toHaveCount(2)
  await expect(page.getByLabel('Style bibliographique', { exact: true })).toHaveValue('ieee')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
  await expect(page.locator('#message')).toContainText('sources Typst exportés')
})

test('imports et remplacements atomiques refusent collisions et références devenues absentes', async ({
  page,
  context,
}) => {
  await create(page)
  await insert(page)
  const peer = await context.newPage()
  await ready(peer, `bibliography-peer-${Date.now()}`)
  await expect(body(peer).locator('.citation-ref')).toContainText('Bourdieu, 1980')
  await files(page).setInputFiles(file('doublon.bib', bib))
  await expect(error(page)).toContainText('dupliquée')
  await expect(page.locator('.bibliography-file-row')).toHaveCount(2)
  await page.getByRole('button', { name: 'Remplacer · livres.bib', exact: true }).click()
  await files(page).setInputFiles(
    file('nouveau.bib', '@book{autre,author={Autre, Anne},title={Un autre ouvrage},year={2024}}'),
  )
  await expect(error(page)).toContainText('Référence citée absente')
  await expect(page.locator('.bibliography-file-row')).toContainText([
    'livres.bib',
    'articles.yaml',
  ])
  await page.getByRole('button', { name: 'Remplacer · livres.bib', exact: true }).click()
  await files(page).setInputFiles(file('livres-corriges.bib', bib.replace('1980},', '1981},')))
  await expect(page.locator('.bibliography-file-row')).toContainText([
    'livres-corriges.bib',
    'articles.yaml',
  ])
  await expect(body(peer).locator('.citation-ref')).toContainText('Bourdieu, 1981')
  await page.getByRole('button', { name: 'Retirer · livres-corriges.bib', exact: true }).click()
  await page
    .locator('#remove-bibliography-file')
    .getByRole('button', { name: 'Retirer', exact: true })
    .click()
  await expect(page.locator('#remove-bibliography-file [role=alert]')).toContainText(
    'Référence citée absente',
  )
  await page.locator('#remove-bibliography-file').press('Escape')
  await expect(page.locator('#metadata-content')).toBeVisible()
  await expect(page.locator('.bibliography-file-row')).toHaveCount(2)
  await peer.close()
})

test('citation dans un widget : même bandeau, recherche au clavier et annulation sans édition', async ({
  page,
}) => {
  await create(page)
  await page.evaluate(() =>
    window.__spike!.editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'quote',
          content: [
            { type: 'paragraph', content: [{ type: 'text', text: 'Dans une citation en bloc.' }] },
            { type: 'quoteAttribution' },
          ],
        },
        { type: 'paragraph' },
      ],
    }),
  )
  await page.evaluate(() => window.__spike!.editor.commands.setTextSelection(8))
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  // Palette and toolbar share the public command; no atom is published before confirmation.
  await page.getByRole('button', { name: 'Citer une référence', exact: true }).click()
  await choose(page, 'bourdieu')
  await band(page).getByRole('combobox', { name: 'Rechercher une référence' }).press('Escape')
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await page.getByRole('button', { name: 'Citer une référence', exact: true }).click()
  await choose(page, 'bourdieu')
  await band(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(body(page).locator('blockquote .citation-ref')).toHaveCount(1)
  await body(page).press('ControlOrMeta+Enter')
  await expect(band(page).getByRole('combobox', { name: 'Rechercher une référence' })).toBeFocused()
  const bandBox = await band(page).boundingBox(),
    widgetBox = await body(page).locator('blockquote').boundingBox()
  expect(bandBox!.width).toBeLessThanOrEqual(widgetBox!.width + 2)
  await band(page).getByRole('combobox', { name: 'Rechercher une référence' }).press('Escape')
  await expect(body(page)).toBeFocused()
})

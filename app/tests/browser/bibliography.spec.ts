import { expectPreviewReady } from './preview-assertions'
import { openRecordCard, confirmRecordDialog } from './record-dialog-controls'
import { confirmNewDocument } from './document-lifecycle'
import { expect, test, type Page } from '@playwright/test'
const bib =
  '@book{bourdieu1980, author={Bourdieu, Pierre}, title={Le sens pratique}, year={1980}, publisher={Minuit}}'
const yaml =
  'durand2020:\n  type: Article\n  title: Le travail collectif\n  author: Durand, Camille\n  date: 2020\n  parent:\n    type: Periodical\n    title: Revue exemple\n'
const tab = (page: Page, id: string) => page.locator(`#metadata-tab-${id}`).click()
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
const band = (page: Page) => page.locator('#citation-properties')
const manager = (page: Page) => page.locator('#bibliography-manager')
const files = (page: Page) => manager(page).getByLabel('Fichiers bibliographiques', { exact: true })
async function manage(page: Page) {
  await page.getByRole('button', { name: 'Gérer les références', exact: true }).click()
  await expect(manager(page)).toBeVisible()
}
async function finishManager(page: Page) {
  await manager(page).getByRole('button', { name: 'Fermer', exact: true }).click()
  await expect(manager(page)).toBeHidden()
}

async function setFiles(
  page: Page,
  value: Parameters<ReturnType<typeof files>['setInputFiles']>[0],
) {
  await expect(manager(page).getByRole('button', { name: 'Importer…', exact: true })).toBeEnabled()
  await files(page).setInputFiles(value)
}
const file = (name: string, contents: string) => ({
  name,
  mimeType: 'text/plain',
  buffer: Buffer.from(contents),
})
async function ready(page: Page, id = 'main', welcome = false) {
  await page.goto(`/?client=${id}${welcome ? '&welcome=1' : ''}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
async function create(page: Page) {
  await ready(page, 'main', true)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog [data-type="article"]').click()
  await confirmNewDocument(page)
  await tab(page, 'titles')
  await openRecordCard(page)
  await page.getByRole('textbox', { name: 'Titre', exact: true }).fill('Article bibliographique')
  await confirmRecordDialog(page)
  await tab(page, 'bibliography')
  await manage(page)
  await setFiles(page, file('livres.bib', bib))
  await page
    .locator('#bibliography-manager')
    .getByRole('button', { name: 'Importer', exact: true })
    .click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(1)
  await setFiles(page, file('articles.yaml', yaml))
  await page
    .locator('#bibliography-manager')
    .getByRole('button', { name: 'Importer', exact: true })
    .click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await expect(manager(page).getByRole('alert')).toHaveCount(0)
  await finishManager(page)
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
  await expect(body(page).locator('.inline-object-label')).toHaveText(
    '(Bourdieu, 1980 ; Durand, 2020)',
  )
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
  await expect(body(page).locator('.inline-object-label')).toHaveText(
    'Bourdieu, 1980 ; Durand, 2020',
  )
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
  await expectPreviewReady(page)
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
  await expect(page.getByLabel('Afficher le sommaire', { exact: true })).toHaveValue('')
  await page.getByLabel('Afficher le sommaire', { exact: true }).selectOption('true')
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
  await expect(page.getByLabel('Afficher le sommaire', { exact: true })).toHaveValue('true')
  await tab(page, 'bibliography')
  await manage(page)
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await finishManager(page)
  await expect(page.getByLabel('Style bibliographique', { exact: true })).toHaveValue('ieee')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
  await expect(page.locator('#message')).toContainText('sources Typst exportés')
})

test('imports natifs, arbitrage de collision et suppression protégée', async ({
  page,
  context,
}) => {
  await create(page)
  await insert(page)
  const peer = await context.newPage()
  await ready(peer, `bibliography-peer-${Date.now()}`)
  await manage(page)
  await setFiles(page, file('doublon.bib', bib))
  await manager(page).getByRole('button', { name: 'Importer', exact: true }).click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await setFiles(
    page,
    file(
      'collision.bib',
      '@book{bourdieu1980,author={Other, Alex},title={A different book},year={1990}}',
    ),
  )
  await expect(manager(page).getByRole('alert')).toContainText('décision de rapprochement')
  await manager(page)
    .getByLabel('Décision · bourdieu1980', { exact: true })
    .selectOption('distinct')
  await manager(page)
    .getByLabel('Clé de citation · bourdieu1980', { exact: true })
    .fill('other1990')
  await expect(manager(page).getByRole('button', { name: 'Importer', exact: true })).toBeEnabled()
  await manager(page).getByRole('button', { name: 'Importer', exact: true }).click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(3)
  const search = manager(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill('bourdieu')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(1)
  await search.press('Enter')
  await manager(page).getByRole('button', { name: 'Supprimer', exact: true }).click()
  const confirm = page.locator('#bibliography-remove')
  await confirm.getByRole('button', { name: 'Supprimer', exact: true }).click()
  await expect(confirm.getByRole('alert')).toContainText('Une citation vise une notice absente')
  await confirm.press('Escape')
  await search.fill('')
  await setFiles(page, file('incomplete.yaml', 'draft: {type: book, title: An incomplete record}'))
  await expect(manager(page)).toContainText('Un champ conseillé est manquant.')
  await manager(page).getByRole('button', { name: 'Importer', exact: true }).click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(4)
  await finishManager(page)
  await expect(body(peer).locator('.citation-ref')).toContainText('Bourdieu, 1980')
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

test('notice privée, fermeture protégée, auteurs réordonnés et validation au clavier', async ({
  page,
}) => {
  await create(page)
  await manage(page)
  await expect(
    manager(page).getByRole('button', { name: 'Créer une notice', exact: true }),
  ).toBeEnabled()
  await manager(page).getByRole('button', { name: 'Créer une notice', exact: true }).click()
  const title = manager(page).getByLabel('Titre', { exact: true })
  await expect(title).toBeFocused()
  await title.fill('Une notice privée')
  await title.press('Escape')
  const leave = page.locator('#bibliography-leave')
  await expect(leave).toBeVisible()
  await leave.getByRole('button', { name: 'Continuer l’édition', exact: true }).click()
  await expect(title).toHaveValue('Une notice privée')
  const authors = manager(page).getByRole('group', { name: 'Auteurs', exact: true })
  for (const [name, given] of [
    ['Martin', 'Camille'],
    ['Durand', 'Alex'],
  ]) {
    await authors.getByRole('button', { name: 'Ajouter une personne', exact: true }).click()
    await authors.getByLabel('Nom ou organisme', { exact: true }).last().fill(name)
    await authors.getByLabel('Prénom', { exact: true }).last().fill(given)
  }
  const handle = authors.locator('.collection-card-handle').last()
  await handle.focus()
  await handle.press('Space')
  await handle.press('ArrowUp')
  await handle.press('Enter')
  await expect(authors.getByLabel('Nom ou organisme', { exact: true }).first()).toHaveValue(
    'Durand',
  )
  await title.dispatchEvent('compositionstart')
  await title.press('ControlOrMeta+Enter')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await title.dispatchEvent('compositionend')
  await title.press('ControlOrMeta+Enter')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(3)
  await finishManager(page)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await manage(page)
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await finishManager(page)
})

test('créer et citer publie un seul lot annulable', async ({ page }) => {
  await create(page)
  await body(page).fill('Une citation nouvelle. ')
  await page.getByRole('button', { name: 'Citer une référence', exact: true }).click()
  await band(page).getByRole('button', { name: 'Créer une notice…', exact: true }).click()
  const title = manager(page).getByLabel('Titre', { exact: true })
  await expect(title).toBeFocused()
  await title.fill('Nouvelle publication')
  await manager(page).getByRole('button', { name: 'Créer et citer', exact: true }).click()
  await expect(manager(page)).toBeHidden()
  await expect(body(page).locator('.citation-ref')).toHaveCount(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(body(page).locator('.citation-ref')).toHaveCount(0)
  await manage(page)
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await finishManager(page)
})

test('une modification concurrente refuse le formulaire et conserve sa saisie', async ({
  page,
  context,
}) => {
  await create(page)
  await manage(page)
  const search = manager(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill('bourdieu')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(1)
  await search.press('Enter')
  const title = manager(page).getByLabel('Titre', { exact: true })
  await expect(title).toHaveValue('Le sens pratique')
  await title.fill('Mon brouillon')
  const peer = await context.newPage()
  await ready(peer, `manager-peer-${Date.now()}`)
  await tab(peer, 'bibliography')
  await manage(peer)
  const other = manager(peer).getByRole('combobox', { name: 'Rechercher une référence' })
  await other.fill('bourdieu')
  await expect(peer.locator('.bibliography-list-row')).toHaveCount(1)
  await other.press('Enter')
  await manager(peer).getByLabel('Titre', { exact: true }).fill('Version de l’autre fenêtre')
  await manager(peer).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(manager(page).getByRole('alert')).toContainText('modifiée ailleurs')
  await manager(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(title).toHaveValue('Mon brouillon')
  await expect(manager(page).getByRole('alert')).not.toBeEmpty()
  await title.press('Escape')
  await page
    .locator('#bibliography-leave')
    .getByRole('button', { name: 'Abandonner', exact: true })
    .click()
  await expect(manager(page)).toBeHidden()
  await peer.close()
})

test('le formulaire bibliographique est récupéré sans publication automatique', async ({
  page,
}) => {
  await create(page)
  await manage(page)
  await expect(
    manager(page).getByRole('button', { name: 'Créer une notice', exact: true }),
  ).toBeEnabled()
  await manager(page).getByRole('button', { name: 'Créer une notice', exact: true }).click()
  await manager(page).getByLabel('Titre', { exact: true }).fill('Notice à reprendre')
  await manager(page)
    .getByLabel('Titre de la publication', { exact: true })
    .fill('Revue du brouillon')
  await changeType(page, 'book')
  await page.evaluate(() => window.__spike!.provider.privateRecovery!.force())
  await page.evaluate(() => window.__spike!.provider.transport.invoke('diagnostic_request_close'))
  const closing = page.locator('dialog.recovery-close-dialog')
  await expect(closing).toBeVisible()
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    closing.getByRole('button', { name: 'Conserver et fermer', exact: true }).click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page
      .locator('.recovery-work-list')
      .getByRole('button', { name: 'Reprendre', exact: true })
      .first()
      .click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
  await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
  const title = manager(page).getByLabel('Titre', { exact: true })
  await expect(title).toHaveValue('Notice à reprendre')
  await expect(
    manager(page).getByRole('combobox', { name: 'Type de notice', exact: true }),
  ).toHaveValue('book')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await title.press('ControlOrMeta+Enter')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(3)
  await finishManager(page)
  await expect(page.locator('#side-content')).toContainText('Toutes les saisies ont été traitées.')
})

test('liste longue et titres tronqués : recherche Unicode et défilements indépendants', async ({
  page,
}) => {
  await create(page)
  await manage(page)
  const contents = Array.from(
    { length: 90 },
    (_, i) =>
      `work${i}: {type: book, title: "Étude ${i} ${'du travail collectif '.repeat(15)}", author: "Auteur${i}, Camille", date: 2026}`,
  ).join('\n')
  await setFiles(page, file('collection.yaml', contents))
  await manager(page).getByRole('button', { name: 'Importer', exact: true }).click()
  await expect(page.locator('.bibliography-list-row')).toHaveCount(92)
  await expect(manager(page).getByRole('button', { name: 'Importer…', exact: true })).toBeEnabled()
  const geometry = await manager(page).evaluate((root) => {
    const list = root.querySelector<HTMLElement>('.bibliography-list')!,
      right = root.querySelector<HTMLElement>('.bibliography-detail')!,
      browser = root.querySelector<HTMLElement>('.bibliography-browser')!,
      search = root.querySelector('input')!
    const controlTop = search.getBoundingClientRect().top,
      rightTop = right.scrollTop
    list.scrollTop = list.scrollHeight
    const title = [
      ...root.querySelectorAll<HTMLElement>('.bibliography-list-row span:last-child'),
    ].find((t) => t.textContent?.startsWith('Étude'))!
    return {
      scrolls: list.scrollTop > 0,
      stable: search.getBoundingClientRect().top === controlTop && right.scrollTop === rightTop,
      confined: browser.getBoundingClientRect().right <= right.getBoundingClientRect().left,
      clipped: title.scrollWidth > title.clientWidth,
    }
  })
  expect(geometry).toEqual({ scrolls: true, stable: true, confined: true, clipped: true })
  const search = manager(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill('Étude 89 Auteur89')
  await expect(page.locator('.bibliography-list-row').first()).toContainText('Étude 89')
  await expect.poll(() => page.locator('.bibliography-list-row').count()).toBeLessThan(92)
  await search.press('ArrowDown')
  await expect(search).toBeFocused()
  await search.press('ArrowUp')
  await search.press('Enter')
  await expect(manager(page).getByLabel('Titre', { exact: true })).toHaveValue(/Étude 89/)
  await page.emulateMedia({ colorScheme: 'dark' })
  await finishManager(page)
})

test('créer depuis une citation groupée conserve les références déjà choisies', async ({
  page,
}) => {
  await create(page)
  await insert(page)
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  await body(page).press('ControlOrMeta+Enter')
  await band(page).getByRole('button', { name: 'Créer une notice…', exact: true }).click()
  await manager(page).getByLabel('Titre', { exact: true }).fill('Troisième ouvrage')
  await manager(page).getByRole('button', { name: 'Créer et citer', exact: true }).click()
  await expect(manager(page)).toBeHidden()
  const items = await page.evaluate(() => {
    let found: any
    window.__spike!.editor.state.doc.descendants((n) => {
      if (n.type.name === 'citation') found = n.attrs.items
    })
    return found
  })
  expect(items).toHaveLength(3)
  expect(items[0].supplement).toBe('p. 23–25')
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
})

async function changeType(page: Page, value: string) {
  const select = manager(page).getByRole('combobox', { name: 'Type de notice', exact: true })
  await select.selectOption(value)
  await expect(select).toHaveValue(value)
  await expect(manager(page).getByRole('button', { name: 'Valider', exact: true })).toBeEnabled()
}

test('changer de type conserve les champs et conteneurs, puis valide une seule édition', async ({
  page,
}) => {
  await create(page)
  await insert(page)
  await manage(page)
  const search = manager(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill('bourdieu')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(1)
  await search.press('Enter')
  await expect(manager(page).getByLabel('Titre', { exact: true })).toHaveValue('Le sens pratique')
  await manager(page).getByLabel('Édition ou version', { exact: true }).fill('Deuxième')
  await changeType(page, 'journal_article')
  await expect(manager(page).getByLabel('Édition ou version', { exact: true })).toHaveCount(0)
  await manager(page).getByLabel('Titre de la publication', { exact: true }).fill('Revue conservée')
  await changeType(page, 'conference_paper')
  await expect(manager(page).getByLabel('Titre de la publication', { exact: true })).toHaveValue('')
  await manager(page).getByLabel('Titre de la publication', { exact: true }).fill('Actes conservés')
  await changeType(page, 'journal_article')
  await expect(manager(page).getByLabel('Titre de la publication', { exact: true })).toHaveValue(
    'Revue conservée',
  )
  await changeType(page, 'book')
  await expect(manager(page).getByLabel('Édition ou version', { exact: true })).toHaveValue(
    'Deuxième',
  )
  await expect(manager(page).getByLabel('Éditeur', { exact: true })).toHaveValue('Minuit')
  const records = () =>
    page.evaluate(() =>
      (window.__spike!.provider.doc.getMap('settings').get('collections') as any)
        .get('references')
        .toJSON(),
    )
  const before = await records()
  await manager(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(manager(page).getByRole('button', { name: 'Fermer', exact: true })).toBeEnabled()
  await finishManager(page)
  const after = await records(),
    id = Object.keys(before).find((id) => before[id].key === 'bourdieu1980')!
  expect(after[id].entry.type).toBe('book')
  expect(after[id].entry.edition).toBe('Deuxième')
  expect(after[id].retained_parents).toHaveLength(2)
  expect(after[id]).not.toHaveProperty('genre')
  expect(after[id]).not.toHaveProperty('profile')
  await expect(body(page).locator('.citation-ref')).toContainText('Bourdieu, 1980')
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(records).toEqual(before)
})

test('un champ invalide refuse le changement de type en conservant le brouillon', async ({
  page,
}) => {
  await create(page)
  await manage(page)
  await expect(
    manager(page).getByRole('button', { name: 'Créer une notice', exact: true }),
  ).toBeEnabled()
  await manager(page).getByRole('button', { name: 'Créer une notice', exact: true }).click()
  await manager(page).getByLabel('Titre', { exact: true }).fill('Mon brouillon')
  const date = manager(page).getByLabel('Date de publication', { exact: true })
  await date.fill('not-a-date')
  const profile = manager(page).getByRole('combobox', { name: 'Type de notice', exact: true })
  await profile.selectOption('book')
  await expect(manager(page).getByRole('alert')).not.toBeEmpty()
  await expect(profile).toHaveValue('journal_article')
  await expect(date).toHaveValue('not-a-date')
  await expect(manager(page).getByLabel('Titre', { exact: true })).toHaveValue('Mon brouillon')
  await date.fill('2026')
  await changeType(page, 'book')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(2)
  await profile.press('Escape')
  await page
    .locator('#bibliography-leave')
    .getByRole('button', { name: 'Abandonner', exact: true })
    .click()
  await expect(manager(page)).toBeHidden()
})

test('volume et numéro hérités restent visibles et modifiables après un changement de type', async ({
  page,
}) => {
  await create(page)
  await manage(page)
  await setFiles(
    page,
    file(
      'native.yaml',
      'nativevolume: {type: article, title: Volume natif, parent: {type: periodical, title: Revue native, volume: 12, issue: 2}}',
    ),
  )
  await manager(page).getByRole('button', { name: 'Importer', exact: true }).click()
  await expect(manager(page).getByRole('button', { name: 'Importer…', exact: true })).toBeEnabled()
  const search = manager(page).getByRole('combobox', { name: 'Rechercher une référence' })
  await search.fill('Volume natif')
  await expect(page.locator('.bibliography-list-row')).toHaveCount(1)
  await search.press('Enter')
  await expect(manager(page).getByLabel('Volume', { exact: true })).toHaveValue('12')
  await expect(manager(page).getByLabel('Numéro', { exact: true })).toHaveValue('2')
  await manager(page).getByLabel('Volume', { exact: true }).fill('13')
  await changeType(page, 'book')
  await changeType(page, 'journal_article')
  await expect(manager(page).getByLabel('Volume', { exact: true })).toHaveValue('13')
  await manager(page).getByRole('button', { name: 'Valider', exact: true }).click()
  await finishManager(page)
  const record = await page.evaluate(
    () =>
      Object.values(
        (window.__spike!.provider.doc.getMap('settings').get('collections') as any)
          .get('references')
          .toJSON(),
      ).find((r: any) => r.key === 'nativevolume') as any,
  )
  expect(record.entry).not.toHaveProperty('volume')
  expect(record.entry.parent[0].volume).toBe(13)
})

test('a late usage error cannot overwrite the next notice or its private input', async ({
  page,
}) => {
  await create(page)
  let release!: () => void
  const held = new Promise<void>((resolve) => {
    release = resolve
  })
  let captured!: () => void
  const started = new Promise<void>((resolve) => {
    captured = resolve
  })
  let delayed = false
  await page.route('**/__spike/invoke', async (route) => {
    const request = route.request().postDataJSON()
    if (
      !delayed &&
      request.command === 'bibliography_query' &&
      request.args?.operation === 'bibliography.usage'
    ) {
      delayed = true
      captured()
      await held
      await route.fulfill({ json: { error: 'obsolete usage failure' } })
    } else await route.continue()
  })
  try {
    await manage(page)
    await started
    const rows = page.locator('.bibliography-list-row')
    await expect(rows).toHaveCount(2)
    await expect(manager(page).getByLabel('Titre', { exact: true })).toBeEditable()
    await rows.nth(1).click()
    await expect(rows.nth(1)).toHaveAttribute('aria-selected', 'true')
    const title = manager(page).getByLabel('Titre', { exact: true })
    await title.fill('Private text must survive a late response')
    const response = page.waitForResponse((response) => {
      const request = response.request().postDataJSON()
      return (
        request?.command === 'bibliography_query' &&
        request.args?.operation === 'bibliography.usage'
      )
    })
    release()
    await (await response).finished()
    await page.unrouteAll({ behavior: 'wait' })
    // Let fetch/JSON and the manager's rejected-query callback settle before
    // a negative assertion, which otherwise could pass before the held error.
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        ),
    )
    await expect(title).toHaveValue('Private text must survive a late response')
    await expect(manager(page)).not.toContainText('obsolete usage failure')
    await title.press('Escape')
    await page
      .locator('#bibliography-leave')
      .getByRole('button', { name: 'Abandonner', exact: true })
      .click()
  } finally {
    release()
  }
})

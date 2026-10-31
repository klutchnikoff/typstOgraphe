import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { expect, test, type Page } from '@playwright/test'
import { confirmNewDocument } from './document-lifecycle'
async function ready(page: Page) {
  await page.goto('/?client=main&welcome=1')
  await page.waitForFunction(() => window.__spike?.ready)
}
async function create(page: Page, kind = 'article') {
  await page.getByRole('button', { name: 'Nouveau document', exact: true }).first().click()
  await page.locator('dialog select').first().selectOption(kind)
  await confirmNewDocument(page)
}
async function closeAndKeep(page: Page) {
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.close"]').click()
  const dialog = page.locator('dialog.recovery-close-dialog')
  await expect(dialog).toBeVisible()
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    dialog.getByRole('button', { name: 'Conserver et fermer', exact: true }).click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
}
test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await ready(page)
})
test(
  'un auteur non validé revient dans son formulaire sans publication ni consommation à l’annulation',
  { tag: '@smoke' },
  async ({ page }) => {
    await create(page)
    await page.locator('#metadata-tab-authors').click()
    await page.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).last().click()
    await page
      .locator('dialog.form-dialog')
      .getByLabel('Nom complet', { exact: true })
      .fill('Auteur à reprendre')
    // Closing the document from a native window remains possible while a form is open.
    await page.evaluate(() => window.__spike!.provider.privateRecovery!.force())
    // Close through the public command, without confirming the author form.
    await page.evaluate(() => window.__spike!.provider.transport.invoke('diagnostic_request_close'))
    const closing = page.locator('dialog.recovery-close-dialog')
    await expect(closing).toBeVisible()
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      closing.getByRole('button', { name: 'Conserver et fermer', exact: true }).click(),
    ])
    await page.waitForFunction(() => window.__spike?.ready)
    await expect(
      page.getByRole('heading', { name: 'Travail à reprendre', exact: true }),
    ).toBeVisible()
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      page
        .locator('.recovery-work-list')
        .getByRole('button', { name: 'Reprendre', exact: true })
        .first()
        .click(),
    ])
    await page.waitForFunction(() => window.__spike?.ready)
    await expect(page.locator('.recovered-document-line')).toBeVisible()
    await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
    const dialog = page.locator('dialog.form-dialog')
    await expect(dialog.getByLabel('Nom complet', { exact: true })).toHaveValue(
      'Auteur à reprendre',
    )
    await dialog.getByLabel('Nom complet', { exact: true }).press('Escape')
    await expect(
      page.getByRole('button', { name: 'Reprendre la saisie', exact: true }),
    ).toBeVisible()
    expect(
      await page.evaluate(() =>
        JSON.stringify(window.__spike!.provider.doc.getMap('settings').toJSON()),
      ),
    ).not.toContain('Auteur à reprendre')
    await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
    await dialog.getByRole('button', { name: 'Valider', exact: true }).click()
    await expect(dialog).toHaveCount(0)
    await expect(page.locator('#side-content')).toContainText(
      'Toutes les saisies ont été traitées.',
    )
    expect(
      await page.evaluate(() =>
        JSON.stringify(window.__spike!.provider.doc.getMap('settings').toJSON()),
      ),
    ).toContain('Auteur à reprendre')
  },
)

test('la note récupérée réutilise le bandeau et n’est publiée qu’à sa validation', async ({
  page,
}) => {
  await create(page)
  await page.locator('[data-command="insert.footnote"]').click()
  await page
    .getByLabel('Texte de la note', { exact: true })
    .fill('Une note à reprendre après fermeture.')
  await closeAndKeep(page)
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
  const input = page.getByLabel('Texte de la note', { exact: true })
  await expect(input).toHaveValue('Une note à reprendre après fermeture.')
  expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).not.toContain(
    'Une note à reprendre',
  )
  await input.press('Escape')
  await expect(page.getByRole('button', { name: 'Reprendre la saisie', exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
  await input.press('ControlOrMeta+Enter')
  await expect(page.locator('#properties')).toBeHidden()
  await expect(page.locator('#side-content')).toContainText('Toutes les saisies ont été traitées.')
  expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toContain(
    'Une note à reprendre',
  )
})

test('un changement de document protège la saisie de l’autre fenêtre avant le délai automatique', async ({
  page,
  context,
}) => {
  await create(page)
  await page.locator('[data-command="insert.footnote"]').click()
  await page.getByLabel('Texte de la note', { exact: true }).fill('Note de la première fenêtre')
  const peer = await context.newPage()
  try {
    await peer.goto('/?client=other-recovery&welcome=1')
    await peer.waitForFunction(() => window.__spike?.ready)
    await peer.locator('#metadata-tab-authors').click()
    await peer.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).last().click()
    await peer
      .locator('dialog.form-dialog')
      .getByLabel('Nom complet', { exact: true })
      .fill('Auteur de l’autre fenêtre')
    await page.getByRole('button', { name: 'Fichier', exact: true }).click()
    await page.locator('[data-command="document.close"]').click()
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      peer.waitForEvent('domcontentloaded'),
      page
        .locator('dialog.recovery-close-dialog')
        .getByRole('button', { name: 'Conserver et fermer', exact: true })
        .click(),
    ])
    await page.waitForFunction(() => window.__spike?.ready)
    const entries = await page.evaluate(() =>
      window.__spike!.provider.transport.invoke<{ id: string; window: string }[]>('work_list'),
    )
    const other = entries.find((e) => e.window === 'other-recovery')!
    expect(other).toBeTruthy()
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      peer.waitForEvent('domcontentloaded'),
      page
        .locator(`[data-recovery-id="${other.id}"]`)
        .getByRole('button', { name: 'Reprendre', exact: true })
        .click(),
    ])
    await page.waitForFunction(() => window.__spike?.ready)
    await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
    await expect(
      page.locator('dialog.form-dialog').getByLabel('Nom complet', { exact: true }),
    ).toHaveValue('Auteur de l’autre fenêtre')
    const remaining = await page.evaluate(() =>
      window.__spike!.provider.transport.invoke<{ window: string }[]>('work_list'),
    )
    expect(remaining.some((e) => e.window === 'main')).toBe(true)
  } finally {
    await peer.close()
  }
})

test('un champ refusé revient dans son onglet et attend une validation explicite', async ({
  page,
}) => {
  await create(page, 'courrier')
  await page.evaluate(async () => {
    const { provider } = window.__spike!
    provider.doc.getMap('settings').set('options.paper', 'invalid-paper')
    await provider.flush().catch(() => {})
  })
  await page.locator('#metadata-tab-header').click()
  await page.getByLabel('Objet', { exact: true }).fill('Objet à reprendre sans publication')
  await closeAndKeep(page)
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page
      .locator('.recovery-work-list')
      .getByRole('button', { name: 'Reprendre', exact: true })
      .first()
      .click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
  const subject = page
    .locator('.recovered-items section')
    .filter({ has: page.getByRole('heading', { name: 'Objet', exact: true }) })
  await subject.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
  const input = page.getByLabel('Objet', { exact: true })
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('Objet à reprendre sans publication')
  expect(
    await page.evaluate(() =>
      JSON.stringify(window.__spike!.provider.doc.getMap('settings').toJSON()),
    ),
  ).not.toContain('Objet à reprendre')
  await input.press('Enter')
  await expect(subject).toHaveCount(0)
  expect(
    await page.evaluate(() =>
      JSON.stringify(window.__spike!.provider.doc.getMap('settings').toJSON()),
    ),
  ).toContain('Objet à reprendre')
  await page.getByRole('button', { name: 'Consulter', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Texte non appliqué', exact: true })).toBeVisible()
})

test('une image remplacée reste privée après reprise et utilise les paramètres ordinaires', async ({
  page,
}) => {
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  // Display the historic recipe fixture, then use the ordinary welcome after closing it.
  await page.evaluate(() => history.replaceState(null, '', '?client=main&welcome=1'))
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await page.locator('#image-file').setInputFiles({
    name: 'private-recovery.svg',
    mimeType: 'image/svg+xml',
    buffer: Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="10"><rect width="12" height="10"/></svg>',
    ),
  })
  const path = await page.evaluate(async () => {
    const provider = window.__spike!.provider
    await provider.privateRecovery!.force()
    const record = await provider.transport.invoke<any>('diagnostic_private_recovery')
    return record.input.forms.find((f: any) => f.form === 'properties').data.data.resource
      .path as string
  })
  await closeAndKeep(page)
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page
      .locator('.recovery-work-list')
      .getByRole('button', { name: 'Reprendre', exact: true })
      .first()
      .click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
  expect(await page.evaluate((path) => window.__spike!.provider.resources.has(path), path)).toBe(
    false,
  )
  await page.getByRole('button', { name: 'Reprendre la saisie', exact: true }).click()
  await expect(page.locator('#replace-image')).toHaveText(
    'Image remplacée — appliquer pour confirmer',
  )
  await page.locator('#apply-properties').click()
  await expect(page.locator('#side-content')).toContainText('Toutes les saisies ont été traitées.')
  expect(await page.evaluate((path) => window.__spike!.provider.resources.has(path), path)).toBe(
    true,
  )
})

test('un brouillon JSON refusé reste isolé et consultable après import', async ({ page }) => {
  const directory = await mkdtemp(path.join(tmpdir(), 'typstographe-recovery-import-'))
  const file = path.join(directory, 'private.json')
  const data = JSON.stringify({
    format: 'typstographe-rejected-draft-v1',
    content: { document: { type: 'unknown', text: 'Texte refusé à conserver' } },
    update: [255],
  })
  try {
    await writeFile(file, data)
    await Promise.all([
      page.waitForEvent('domcontentloaded'),
      page.evaluate(
        (path) =>
          window.__spike!.provider.transport.invoke('work_import', {
            path,
            revision: window.__spike!.provider.revision,
          }),
        file,
      ),
    ])
    await page.waitForFunction(() => window.__spike?.ready)
    expect(
      await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON())),
    ).not.toContain('Texte refusé')
    await page.getByRole('button', { name: 'Consulter', exact: true }).click()
    const dialog = page.getByRole('dialog', { name: 'Texte non appliqué', exact: true })
    await expect(
      dialog.getByRole('textbox', { name: 'Contenu récupéré', exact: true }),
    ).toHaveValue('Texte refusé à conserver')
    await dialog.getByRole('textbox').press('Escape')
    await expect(page.getByRole('button', { name: 'Consulter', exact: true })).toBeVisible()
    expect(await readFile(file, 'utf8')).toBe(data)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

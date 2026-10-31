import { expect, test, type Page } from '@playwright/test'
import { confirmNewDocument } from './document-lifecycle'

const record = (page: Page) =>
  page.evaluate(async () => {
    const provider = window.__spike!.provider
    await provider.privateRecovery!.flush()
    return provider.transport.invoke<any>('diagnostic_private_recovery')
  })
async function ready(page: Page, client = 'main') {
  await page.goto('/?client=' + client)
  await page.waitForFunction(
    () => window.__spike?.ready && !!window.__spike.provider.privateRecovery,
  )
}
async function article(page: Page) {
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await confirmNewDocument(page)
  await page.locator('#metadata-tab-authors').click()
}
async function author(page: Page, name: string) {
  await page.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).last().click()
  await page.locator('dialog.form-dialog').getByLabel('Nom complet', { exact: true }).fill(name)
}
test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await ready(page)
})

test(
  'les notes et images non validées sont conservées sans modification documentaire',
  { tag: '@smoke' },
  async ({ page }) => {
    const before = await page.evaluate(() => window.__spike!.provider.revision)
    await page.locator('.footnote-ref .inline-object-label').click()
    await page.getByLabel('Texte de la note', { exact: true }).fill('Private footnote é😀')
    const note = await record(page)
    expect(JSON.stringify(note.input.forms)).toContain('Private footnote é😀')
    expect(JSON.stringify(note.confirmed.document)).not.toContain('Private footnote é😀')
    expect(note.confirmedRevision).toBe(before)
    await page.locator('#cancel-properties').click()
    await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
    await page.locator('#image-file').setInputFiles({
      name: 'private.svg',
      mimeType: 'image/svg+xml',
      buffer: Buffer.from(
        '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>',
      ),
    })
    await expect(page.locator('#replace-image')).toHaveText(
      'Image remplacée — appliquer pour confirmer',
    )
    const image = await record(page)
    const form = image.input.forms.find((f: any) => f.form === 'properties')
    expect(form.data.data.resource.data).toBeTruthy()
    expect(
      image.confirmed.resources.some((r: any) => r.path === form.data.data.resource.path),
    ).toBe(false)
    await page.locator('#cancel-properties').click()
    expect((await record(page)).input.forms).toEqual([])
  },
)

test('les formulaires Article restent privés et isolés entre fenêtres, puis Échap les abandonne', async ({
  page,
  context,
}) => {
  await article(page)
  const peer = await context.newPage()
  try {
    await ready(peer, 'draft-peer')
    await peer.locator('#metadata-tab-affiliations').click()
    await peer.getByRole('button', { name: 'Ajouter · Affiliation', exact: true }).last().click()
    await peer
      .locator('dialog.form-dialog')
      .getByLabel('Signature officielle', { exact: true })
      .fill('Private affiliation')
    await author(page, 'Unconfirmed author')
    const main = await record(page),
      other = await record(peer)
    expect(
      main.input.forms.find((f: any) => f.context.collection === 'authors').data.data.record.name,
    ).toBe('Unconfirmed author')
    expect(JSON.stringify(main.input.forms)).toContain('Unconfirmed author')
    expect(JSON.stringify(main.input.forms)).not.toContain('Private affiliation')
    expect(JSON.stringify(other.input.forms)).toContain('Private affiliation')
    expect(JSON.stringify(other.input.forms)).not.toContain('Unconfirmed author')
    expect(JSON.stringify(main.confirmed.settings)).not.toContain('Unconfirmed author')
    expect(JSON.stringify(other.confirmed.settings)).not.toContain('Private affiliation')
    await page
      .locator('dialog.form-dialog')
      .getByLabel('Nom complet', { exact: true })
      .press('Escape')
    expect((await record(page)).input.forms).toEqual([])
    expect(JSON.stringify((await record(peer)).input.forms)).toContain('Private affiliation')
  } finally {
    await peer.close()
  }
})

test('la préédition d’un formulaire ne remplace pas la dernière saisie validée au clavier', async ({
  page,
}) => {
  await article(page)
  await author(page, 'Before composition')
  await record(page)
  await page
    .locator('dialog.form-dialog')
    .getByLabel('Nom complet', { exact: true })
    .evaluate((input) => {
      input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }))
      ;(input as HTMLInputElement).value = 'Preedit に'
      input.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing: true }))
    })
  expect(JSON.stringify((await record(page)).input.forms)).not.toContain('Preedit')
  await page
    .locator('dialog.form-dialog')
    .getByRole('button', { name: 'Valider', exact: true })
    .click()
  await expect(page.locator('dialog.form-dialog')).toBeVisible()
  await page
    .locator('dialog.form-dialog')
    .getByLabel('Nom complet', { exact: true })
    .evaluate((input) => {
      ;(input as HTMLInputElement).value = 'Validated にほん'
      input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }))
      input.dispatchEvent(new InputEvent('input', { bubbles: true }))
    })
  expect(JSON.stringify((await record(page)).input.forms)).toContain('Validated にほん')
})

test('une fenêtre refusée protège son texte et ses champs tandis que l’autre fenêtre continue', async ({
  page,
  context,
}) => {
  const peer = await context.newPage()
  try {
    await ready(peer, 'confirmed-peer')
    await page.evaluate(async () => {
      const { provider, editor } = window.__spike!
      provider.doc.getMap('settings').set('options.paper', 'invalid-paper')
      await provider.flush().catch(() => {})
      editor.commands.insertContent('Private rejected text ')
    })
    await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeVisible()
    await page.locator('#metadata-tab-header').click()
    await page.getByLabel('Objet', { exact: true }).fill('Private pending subject')
    await page.getByLabel('Objet', { exact: true }).dispatchEvent('blur')
    await peer.evaluate(async () => {
      const { editor, provider } = window.__spike!
      editor.commands.insertContent('Confirmed peer text ')
      await provider.flush()
    })
    const saved = await record(page)
    expect(saved.input.rejected).toBe(true)
    expect(JSON.stringify(saved.input.fields)).toContain('Private pending subject')
    expect(JSON.stringify(saved.input.document)).toContain('Private rejected text')
    expect(JSON.stringify(saved.confirmed.document)).toContain('Confirmed peer text')
    expect(JSON.stringify(saved.confirmed)).not.toContain('Private rejected text')
    expect(JSON.stringify(saved.confirmed)).not.toContain('Private pending subject')
  } finally {
    await peer.close()
  }
})

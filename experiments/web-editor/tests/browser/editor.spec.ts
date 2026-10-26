import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})
async function ready(page: Page, id: string) {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('#pages img').first()).toBeVisible()
}
async function append(page: Page, text: string) {
  await page.evaluate((text) => {
    const e = window.__spike!.editor
    e.commands.insertContentAt(e.state.doc.content.size, {
      type: 'paragraph',
      content: [{ type: 'text', text }],
    })
  }, text)
}
test('rendu réel, bandeau de note, légende et sélection de tableau au clavier', async ({
  page,
}, testInfo) => {
  await ready(page, 'main')
  await expect(page.getByRole('textbox', { name: 'Corps du document' })).toContainText(
    'Madame, Monsieur',
  )
  await page.locator('.footnote-ref').click()
  await page
    .getByLabel('Texte de la note', { exact: true })
    .fill('Note relue : ê, é, Noël, にほん.')
  await page.getByLabel('Texte de la note', { exact: true }).press('Enter')
  await expect(page.locator('#properties')).toBeHidden()
  await page.locator('figcaption').click()
  await page.keyboard.press('Home')
  await page.keyboard.type('Légende : ')
  await expect(page.locator('figcaption')).toContainText('Légende :')
  await page.locator('td').first().click()
  await expect
    .poll(() =>
      page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.textContent),
    )
    .toBe('Places assises')
  await page.keyboard.press('Tab')
  await expect
    .poll(() =>
      page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.textContent),
    )
    .toBe('30')
  await page.keyboard.type(' vérifié')
  await expect(page.locator('.table-tools')).toBeVisible()
  await expect(page.locator('table')).toContainText('vérifié')
  await page.getByRole('button', { name: 'Ajouter une ligne', exact: true }).click()
  await expect(page.locator('tr')).toHaveCount(4)
  await expect(page.locator('#render-status')).not.toContainText('corriger')
  await page.waitForFunction(() =>
    /page/.test(document.querySelector('#render-status')!.textContent || ''),
  )
  await page.screenshot({ path: testInfo.outputPath('editor-light.png'), fullPage: true })
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.screenshot({ path: testInfo.outputPath('editor-dark.png'), fullPage: true })
})
test('deux vues fusionnent hors ligne, annulation locale, sauvegarde et commande Rust', async ({
  page,
  context,
}) => {
  await ready(page, 'main')
  const peer = await context.newPage()
  await ready(peer, `peer-${Date.now()}`)
  await peer.getByRole('button', { name: 'Connecté', exact: true }).click()
  await expect(peer.getByRole('button', { name: 'Hors connexion', exact: true })).toBeVisible()
  const token = String(Date.now())
  await append(peer, `ALEX-${token}`)
  await append(page, `CAMILLE-${token}`)
  await peer.getByRole('button', { name: 'Hors connexion', exact: true }).click()
  await expect(page.locator('.tiptap')).toContainText(`ALEX-${token}`)
  await expect(peer.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await peer.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(page.locator('.tiptap')).not.toContainText(`ALEX-${token}`)
  await expect(page.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await page.getByRole('button', { name: 'Essai Rust', exact: true }).click()
  await expect(peer.locator('.tiptap')).toContainText('Cette phrase a été préparée depuis Rust')
  await page.getByRole('button', { name: 'Enregistrer', exact: true }).click()
  await expect(page.locator('#message')).toContainText('Session enregistrée', { timeout: 30_000 })
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await page.getByRole('button', { name: 'Exporter', exact: true }).click()
  await expect(page.locator('#message')).toContainText('sources Typst exportés', {
    timeout: 30_000,
  })
  await peer.close()
})

import { confirmNewDocument } from './document-lifecycle'
import { expect, test, type Page } from '@playwright/test'
import { readFile } from 'node:fs/promises'

const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
async function ready(page: Page, id: string) {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
test.beforeEach(async ({ request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
})

test(
  'le propriétaire Rust regroupe la frappe et annule uniquement les contributions de sa fenêtre',
  { tag: '@smoke' },
  async ({ page, context }) => {
    await ready(page, 'main')
    await body(page).fill('Base')
    await page.evaluate(async () => {
      await window.__spike!.provider.clearHistory()
    })
    await body(page).press('End')
    await page.keyboard.type(' à relire 日本')
    await page.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    expect(await page.evaluate(() => window.__spike!.provider.historyState.undo_steps)).toBe(1)
    // Native zones bind only ySync; there is no second Yjs undo manager.
    expect(
      await page.evaluate(() =>
        window.__spike!.editor.extensionManager.extensions.some(
          (extension) => extension.name === 'collaboration',
        ),
      ),
    ).toBe(false)
    const peer = await context.newPage()
    await ready(peer, 'peer-history')
    await body(peer).press('ControlOrMeta+End')
    await page.evaluate(() => window.__spike!.provider.stopCapturing())
    await peer.keyboard.type('Contribution distante')
    await peer.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    await expect(body(page)).toContainText('Contribution distante')
    await body(page).press('ControlOrMeta+z')
    await expect(body(page)).not.toContainText('à relire')
    await expect(body(peer)).toContainText('Contribution distante')
    await expect(body(page)).toBeFocused()
    await body(page).press('ControlOrMeta+Shift+z')
    await expect(body(peer)).toContainText('à relire 日本')
    await peer.close()
  },
)

test(
  'un refus conserve le brouillon entier, suspend les envois et permet un retour explicite',
  { tag: '@smoke' },
  async ({ page, context }) => {
    await ready(page, 'main')
    await body(page).fill('Texte confirmé')
    await page.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    const peer = await context.newPage()
    await ready(peer, 'peer-refusal')
    await page.evaluate(async () => {
      const provider = window.__spike!.provider
      provider.doc.getMap('settings').set('options.paper', 'invalid-paper')
      await provider.flush().catch(() => {})
    })
    await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeVisible()
    await body(page).fill('Brouillon non confirmé conservé')
    await expect(body(peer)).toContainText('Texte confirmé')
    await body(peer).fill('Nouvelle version confirmée du collègue')
    await peer.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    await expect(body(page)).toContainText('Brouillon non confirmé conservé')
    const downloading = page.waitForEvent('download')
    await page.getByRole('button', { name: 'Exporter le brouillon', exact: true }).click()
    const download = await downloading
    const draft = JSON.parse(await readFile((await download.path())!, 'utf8'))
    expect(draft.format).toBe('typstographe-rejected-draft-v1')
    expect(JSON.stringify(draft.content.document)).toContain('Brouillon non confirmé conservé')
    expect(draft.update.length).toBeGreaterThan(0)
    await page.getByRole('button', { name: 'Revenir à la version confirmée', exact: true }).click()
    await page.getByRole('dialog').getByRole('button', { name: 'Annuler', exact: true }).click()
    await expect(body(page)).toContainText('Brouillon non confirmé conservé')
    await page.getByRole('button', { name: 'Revenir à la version confirmée', exact: true }).click()
    await page.getByRole('button', { name: 'Abandonner le brouillon', exact: true }).click()
    await page.waitForFunction(() => window.__spike?.ready && !window.__spike.provider.rejected)
    await expect(body(page)).toContainText('Nouvelle version confirmée du collègue')
    await body(page).press('End')
    await page.keyboard.type(' — reprise')
    await page.evaluate(async () => {
      await window.__spike!.provider.flush()
    })
    await expect(body(peer)).toContainText('reprise')
    await peer.close()
  },
)

test('la préédition reste locale jusqu’à compositionend, puis les deux fenêtres convergent', async ({
  page,
  context,
}) => {
  await ready(page, 'main')
  await body(page).fill('Base ')
  await page.evaluate(async () => {
    await window.__spike!.provider.clearHistory()
  })
  const peer = await context.newPage()
  await ready(peer, 'peer-ime')
  await body(page).press('End')
  await body(page).dispatchEvent('compositionstart', { data: '' })
  await page.evaluate(() => window.__spike!.editor.commands.insertContent('にほ'))
  await expect(body(page)).toContainText('にほ')
  expect(await page.evaluate(() => window.__spike!.provider.composing)).toBe(true)
  await expect(body(peer)).not.toContainText('にほ')
  await page.evaluate(() => window.__spike!.editor.commands.insertContent('ん'))
  await body(page).dispatchEvent('compositionend', { data: 'にほん' })
  await expect(body(peer)).toContainText('にほん')
  await page.evaluate(async () => {
    await window.__spike!.provider.flush()
  })
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
  await body(page).press('ControlOrMeta+z')
  await expect(body(peer)).not.toContainText('にほん')
  await peer.close()
})

test('la validation IME survit au retrait immédiat de la zone riche d’en-tête', async ({
  page,
}) => {
  await ready(page, 'main')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await confirmNewDocument(page)
  await page.locator('#metadata-tab-titles').click()
  const title = page.getByRole('textbox', { name: 'Titre', exact: true })
  await title.fill('Titre ')
  await title.press('End')
  await title.dispatchEvent('compositionstart', { data: '' })
  await page.keyboard.insertText('にほん')
  await page.evaluate(() => {
    const title = document.querySelector<HTMLElement>('[aria-label="Titre"][contenteditable]')!
    title.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: 'にほん' }))
    document.getElementById('metadata-tab-general')!.click()
  })
  await expect.poll(() => page.evaluate(() => window.__spike!.provider.composing)).toBe(false)
  await page.evaluate(async () => {
    await window.__spike!.provider.flush()
  })
  await page.locator('#metadata-tab-titles').click()
  await expect(title).toContainText('にほん')
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
})

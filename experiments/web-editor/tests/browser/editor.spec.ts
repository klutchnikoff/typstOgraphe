import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import { writeFile } from 'node:fs/promises'
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
  await page.screenshot({ path: testInfo.outputPath('editor-overview.png') })
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

test('sélection entre blocs et saisie sur un document plus long', async ({ page }, testInfo) => {
  test.setTimeout(120_000)
  await ready(page, 'main')
  // Sélection réelle du navigateur, puis remplacement entre deux paragraphes.
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: ['Premier passage.', 'Second passage.'].map((text) => ({
        type: 'paragraph',
        content: [{ type: 'text', text }],
      })),
    })
    e.commands.setTextSelection(1)
    e.view.focus()
  })
  await page.keyboard.down('Shift')
  for (let i = 0; i < 20; i++) await page.keyboard.press('ArrowRight')
  await page.keyboard.up('Shift')
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__spike!.editor.state.selection.$from.parent !==
          window.__spike!.editor.state.selection.$to.parent,
      ),
    )
    .toBe(true)
  await page.keyboard.type('Remplacement')
  await expect(page.locator('.tiptap')).toContainText('Remplacement')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: Array.from({ length: 100 }, (_, index) => ({
        type: 'paragraph',
        content: [
          {
            type: 'text',
            text: `Paragraphe ${index + 1}. Une rédaction structurée doit rester fluide lorsque le document grandit. `.repeat(
              3,
            ),
          },
        ],
      })),
    })
    e.commands.focus('end')
  })
  await expect
    .poll(() => page.locator('#render-status').textContent(), { timeout: 60_000 })
    .toMatch(/\d+ pages? · \d+ ms/)
  // Du keydown jusqu'à la prochaine frame après la transaction : mesure graphique,
  // sans compter les allers-retours entre Playwright et le navigateur.
  await page.evaluate(() => {
    const samples: number[] = []
    let start = 0
    const onKey = (event: KeyboardEvent) => {
      if (event.key.length === 1) start = performance.now()
    }
    const onUpdate = () => {
      const timestamp = start
      start = 0
      if (timestamp) requestAnimationFrame(() => samples.push(performance.now() - timestamp))
    }
    const e = window.__spike!.editor
    e.view.dom.addEventListener('keydown', onKey)
    e.on('update', onUpdate)
    Object.assign(window, {
      __latency: {
        samples,
        stop: () => {
          e.view.dom.removeEventListener('keydown', onKey)
          e.off('update', onUpdate)
        },
      },
    })
  })
  await page.keyboard.type('Un test de saisie après cent paragraphes.', { delay: 30 })
  await page.waitForTimeout(100)
  const result = await page.evaluate(() => {
    const metrics = (window as unknown as { __latency: { samples: number[]; stop: () => void } })
      .__latency
    metrics.stop()
    const values = metrics.samples.sort((a, b) => a - b)
    return {
      paragraphs: window.__spike!.editor.getJSON().content?.length,
      signs: window.__spike!.editor.getText().length,
      samples: values.length,
      medianMs: values[Math.floor(values.length / 2)],
      p95Ms: values[Math.floor(values.length * 0.95)],
      maxMs: values.at(-1),
      preview: document.querySelector('#render-status')!.textContent,
      userAgent: navigator.userAgent,
      // Indicateur Chromium seulement, ni le DOM ni le RSS total de Tauri/WebKit.
      approximateJSHeapMiB: (performance as Performance & { memory?: { usedJSHeapSize: number } })
        .memory?.usedJSHeapSize
        ? (performance as Performance & { memory: { usedJSHeapSize: number } }).memory
            .usedJSHeapSize /
          1024 /
          1024
        : null,
    }
  })
  expect(result.samples).toBeGreaterThan(10)
  const metricsPath = testInfo.outputPath('typing-metrics.json')
  await writeFile(metricsPath, JSON.stringify(result, null, 2))
  await testInfo.attach('typing-metrics', { path: metricsPath, contentType: 'application/json' })
  await page.screenshot({ path: testInfo.outputPath('long-document.png') })
})

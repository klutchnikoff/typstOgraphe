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
  await expect(page.locator('#pages .preview-page').first()).toBeVisible()
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
test('langue documentaire indépendante de l’interface et correction native désactivée dans les widgets', async ({
  page,
}) => {
  await ready(page, 'main')
  const editor = page.locator('.tiptap')
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr')
  await expect(editor).toHaveAttribute('lang', 'fr')
  await expect(editor).toHaveAttribute('spellcheck', 'false')
  await expect(editor).toHaveAttribute('autocorrect', 'off')
  // Captions, list items and cells inherit the same policy as the body.
  for (const selector of ['figcaption', 'li p', 'td p'])
    expect(
      await editor
        .locator(selector)
        .first()
        .evaluate((node) => (node as HTMLElement).spellcheck),
    ).toBe(false)
  await page.getByRole('button', { name: 'Général', exact: true }).click()
  await page.getByLabel('Langue du document', { exact: true }).selectOption('en')
  await expect(editor).toHaveAttribute('lang', 'en')
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr')
  await page.getByLabel('Langue du document', { exact: true }).selectOption('fr')
  await expect(editor).toHaveAttribute('lang', 'fr')
})
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
  await page.emulateMedia({ colorScheme: 'dark' })
})
test('deux vues fusionnent hors ligne, annulation locale, sauvegarde et commande Rust', async ({
  page,
  context,
}) => {
  await ready(page, 'main')
  const peer = await context.newPage()
  await ready(peer, `peer-${Date.now()}`)
  await peer.getByRole('button', { name: 'Affichage', exact: true }).click()
  await peer.locator('[data-command="ui.connection"]').click()
  await expect(peer.locator('#connection')).toHaveText('Hors connexion')
  const token = String(Date.now())
  await append(peer, `ALEX-${token}`)
  await append(page, `CAMILLE-${token}`)
  await peer.getByRole('button', { name: 'Affichage', exact: true }).click()
  await peer.locator('[data-command="ui.connection"]').click()
  await expect(page.locator('.tiptap')).toContainText(`ALEX-${token}`)
  await expect(peer.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await peer.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(page.locator('.tiptap')).not.toContainText(`ALEX-${token}`)
  await expect(page.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await page.getByRole('button', { name: 'Essai Rust', exact: true }).click()
  await expect(peer.locator('.tiptap')).toContainText('Cette phrase a été préparée depuis Rust')
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.save"]').click()
  await expect(page.locator('#message')).toContainText('Session enregistrée', { timeout: 30_000 })
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('.tiptap')).toContainText(`CAMILLE-${token}`)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.export"]').click()
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
  await expect
    .poll(() => page.locator('#render-status').textContent(), { timeout: 60_000 })
    .toMatch(/\d+ pages? · \d+ ms/)
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
})

test('capacités reprises, recherche exacte et sélection synchronisée avec le rendu', async ({
  page,
}) => {
  await ready(page, 'main')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'Bonjour ' },
            { type: 'text', text: 'collègue', marks: [{ type: 'bold' }] },
            { type: 'text', text: '. Bonjour encore.' },
          ],
        },
        {
          type: 'quote',
          content: [
            { type: 'paragraph', content: [{ type: 'text', text: 'Une citation sobre.' }] },
            { type: 'quoteAttribution', content: [{ type: 'text', text: 'Source interne' }] },
          ],
        },
        {
          type: 'descriptionList',
          content: [
            {
              type: 'descriptionItem',
              content: [
                { type: 'descriptionTerm', content: [{ type: 'text', text: 'Objet' }] },
                {
                  type: 'descriptionDefinition',
                  content: [{ type: 'text', text: 'Organisation de la réunion.' }],
                },
              ],
            },
          ],
        },
        {
          type: 'postscript',
          content: [{ type: 'text', text: 'Merci de confirmer votre présence.' }],
        },
        { type: 'attachment', content: [{ type: 'text', text: 'Ordre du jour' }] },
      ],
    })
    e.commands.setTextSelection({ from: 1, to: 8 })
  })
  await expect(page.locator('#pages svg polygon').first()).toBeVisible({ timeout: 30000 })
  await page.getByRole('button', { name: 'Rechercher', exact: true }).click()
  await page.getByRole('searchbox', { name: 'Rechercher', exact: true }).fill('bonjour')
  await expect(page.locator('.search-hit')).toHaveCount(2)
  await page.locator('.search-hit').last().click()
  expect(
    await page.evaluate(() => {
      const { from, to } = window.__spike!.editor.state.selection
      return to - from
    }),
  ).toBe(7)
  await page.locator('.search-hit').first().click()
  expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(1)
  await page.getByRole('textbox', { name: 'Remplacer par', exact: true }).fill('<suite>')
  await page.getByRole('button', { name: 'Remplacer', exact: true }).click()
  await expect(page.locator('.tiptap')).toContainText('<suite>')
  await expect(page.locator('.search-hit')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await expect(page.locator('.search-hit')).toHaveCount(0)
  await page.locator('dd').click()
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name))
    .toBe('descriptionDefinition')
  await page.keyboard.press('Control+Backspace')
  await expect(page.locator('dl')).toHaveCount(0)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(page.locator('dl')).toHaveCount(1)
})

test('raccourcis textuels Typst et séparation des paragraphes', async ({ page }) => {
  await ready(page, 'main')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.clearContent()
    e.commands.focus('start')
  })
  await page.keyboard.type('= Titre')
  await expect(page.locator('.tiptap h1')).toHaveText('Titre')
  await page.keyboard.press('Enter')
  await page.keyboard.type('*Important*')
  await expect(page.locator('.tiptap strong')).toHaveText('Important')
  await page.keyboard.press('Enter')
  await page.keyboard.type('suite')
  await expect(page.locator('.tiptap p')).toHaveCount(1)
  await page.keyboard.press('Enter')
  await page.keyboard.press('Enter')
  await page.keyboard.type('Autre paragraphe')
  await expect(page.locator('.tiptap p')).toHaveCount(2)
})

test('accueil sans document et création depuis les types et modèles disponibles', async ({
  page,
}) => {
  await page.goto('/?client=main&welcome=1')
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('.welcome')).toBeVisible()
  await expect(page.locator('main')).toBeHidden()
  await page.locator('#create-document').click()
  await expect(page.locator('dialog')).toBeVisible()
  await expect(page.locator('dialog select').first().locator('option')).toHaveCount(1)
  await expect(page.locator('dialog select').nth(1).locator('option')).toHaveCount(1)
  await page.getByRole('button', { name: 'Créer', exact: true }).click()
  await expect(page.locator('main')).toBeVisible()
  await expect(page.locator('.tiptap')).not.toContainText('Valmont')
  await expect(page.locator('.tiptap')).not.toContainText('Madame, Monsieur')
  await page.getByRole('button', { name: 'Informations', exact: true }).click()
  await page.getByLabel('Objet', { exact: true }).fill('Courrier créé depuis l’accueil')
  await expect(page.locator('#pages .preview-page').first()).toBeVisible({ timeout: 30000 })
})

test('préférences globales enregistrées explicitement et retrouvées après réouverture', async ({
  page,
}) => {
  await ready(page, 'main')
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  await page.getByLabel('Apparence', { exact: true }).selectOption('dark')
  await page.getByLabel('Police de l’éditeur', { exact: true }).selectOption('sans')
  await page.getByLabel('Taille', { exact: true }).fill('21')
  await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-appearance', 'dark')
  await expect(page.locator('.tiptap')).toHaveCSS('font-size', '21px')
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('html')).toHaveAttribute('data-appearance', 'dark')
  await expect(page.locator('.tiptap')).toHaveCSS('font-size', '21px')
})

test('listes au clavier et dans la barre d’outils, bascule et sortie du widget', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await ready(page, 'main')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.clearContent()
    e.commands.focus('start')
  })
  await page.getByRole('button', { name: 'Listes', exact: true }).click()
  await page.locator('[data-command="block.bullet"]').click()
  await expect(page.locator('.widget-bulletList')).toHaveCount(1)
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.keyboard.type('Premier')
  await expect(page.locator('.widget-bulletList li p')).toHaveText('Premier')
  await page.keyboard.press('Enter')
  await page.keyboard.type('Second')
  await expect(page.locator('.widget-bulletList li')).toHaveCount(2)
  await page.keyboard.press('Tab')
  await expect(page.locator('.widget-bulletList .widget-bulletList')).toHaveCount(0)
  await page.getByRole('button', { name: 'Listes', exact: true }).click()
  await page.locator('[data-command="block.numbered"]').click()
  await expect(page.locator('.widget-orderedList li')).toHaveCount(2)
  await expect(page.locator('.widget-bulletList')).toHaveCount(0)
  await page.evaluate(() => window.__spike!.editor.commands.selectAll())
  await page.getByRole('button', { name: 'Listes', exact: true }).click()
  await page.locator('[data-command="block.numbered"]').click()
  await expect(page.locator('.content-widget')).toHaveCount(0)
  await expect(page.locator('.tiptap')).toContainText('Premier')
  await expect(page.locator('.tiptap')).toContainText('Second')

  for (const [prefix, kind] of [
    ['- ', 'bulletList'],
    ['1. ', 'orderedList'],
  ]) {
    await page.evaluate(() => {
      const e = window.__spike!.editor
      e.commands.clearContent()
      e.commands.focus('start')
    })
    await page.keyboard.type(prefix + 'Un')
    await expect(page.locator(`.widget-${kind} li`)).toHaveText('Un')
    await page.keyboard.press('Enter')
    await page.keyboard.type('Deux')
    await expect(page.locator(`.widget-${kind} li`)).toHaveCount(2)
    await page.keyboard.press('Enter')
    await page.keyboard.press('Enter')
    await page.keyboard.type('Après la liste')
    await expect(page.locator('.tiptap > p')).toHaveText('Après la liste')
    await expect(page.locator(`.widget-${kind} li`)).toHaveCount(2)
    await page.waitForFunction(() =>
      /page/.test(document.querySelector('#render-status')!.textContent || ''),
    )
    await expect(page.locator('#message')).not.toHaveClass(/error/)
  }
  await page.evaluate(() => window.__spike!.editor.commands.clearContent())
  await page.getByRole('button', { name: 'Listes', exact: true }).click()
  await page.locator('[data-command="block.description"]').click()
  await page.locator('dt').click()
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name))
    .toBe('descriptionTerm')
  await page.keyboard.type('Terme')
  await page.keyboard.press('Tab')
  await page.keyboard.type('Définition')
  await expect(page.locator('dt')).toHaveText('Terme')
  await expect(page.locator('dd')).toHaveText('Définition')
  await page.keyboard.press('Enter')
  await page.keyboard.type('Autre terme')
  await expect(page.locator('dt')).toHaveCount(2)
  await expect(page.locator('dt').last()).toHaveText('Autre terme')
  expect(errors).toEqual([])
})

test('menus annulés au clic extérieur et avec Échap, sans modification du document', async ({
  page,
}) => {
  await ready(page, 'main')
  const before = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
  for (const label of ['Titres', 'Listes', 'Fichier']) {
    await page.getByRole('button', { name: label, exact: true }).click()
    await expect(page.locator('.tool-menu[open]')).toHaveCount(1)
    await page.locator('.tiptap p').first().click()
    await expect(page.locator('.tool-menu[open]')).toHaveCount(0)
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
  }
  await page.getByRole('button', { name: 'Titres', exact: true }).click()
  await page.getByRole('button', { name: 'Listes', exact: true }).click()
  await expect(page.locator('.tool-menu[open]')).toHaveCount(1)
  await expect(page.getByRole('button', { name: 'Titre 1', exact: true })).toBeHidden()
  await page.keyboard.press('Escape')
  await expect(page.locator('.tool-menu[open]')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Listes', exact: true })).toBeFocused()
  expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
})

test('colonne éditable sans cadre et actions groupées à droite', async ({ page }, testInfo) => {
  await ready(page, 'main')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent('<p>Un courrier très court.</p>')
    e.commands.focus('end')
  })
  await expect(page.locator('.tiptap')).toHaveCSS('outline-style', 'none')
  const area = await page.locator('#editor-scroll').boundingBox()
  const editable = await page.locator('.tiptap').boundingBox()
  expect(editable!.x).toBe(area!.x)
  expect(editable!.width).toBe(area!.width)
  expect(editable!.height).toBeGreaterThanOrEqual(area!.height)
  await page.locator('.tiptap').click({ position: { x: 90, y: area!.height - 30 } })
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.keyboard.type(' Suite.')
  await expect(page.locator('.tiptap')).toContainText('Suite.')
  await expect(page.locator('#render-status')).toHaveText(/^\d+ pages? ·/)
  await page.emulateMedia({ colorScheme: 'dark' })

  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  const dialog = page.locator('dialog')
  await expect(dialog).toBeVisible()
  const buttons = dialog.locator('.panel-actions > button')
  await expect(buttons).toHaveText(['Abandonner', 'Annuler', 'Enregistrer'])
  await expect(buttons.first()).toHaveClass('danger')
  await expect(buttons.last()).toHaveClass('primary')
  const row = dialog.locator('.panel-actions')
  await expect(row).toHaveCSS('justify-content', 'flex-end')
  const last = await buttons.last().boundingBox()
  const bounds = await row.boundingBox()
  expect(Math.abs(last!.x + last!.width - bounds!.x - bounds!.width)).toBeLessThan(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).last().click()
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  await expect(page.locator('#side-content .panel-actions')).toHaveCSS(
    'justify-content',
    'flex-end',
  )
  await expect(page.locator('#side-content .panel-actions')).toHaveCSS('flex-direction', 'row')
  await expect(page.locator('#side-content .panel-actions > button').last()).toHaveClass('primary')
})

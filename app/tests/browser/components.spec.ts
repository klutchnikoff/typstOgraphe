import { expect, test } from '@playwright/test'
import fixture from '../../fixture.json' with { type: 'json' }

test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('#pages .preview-page').first()).toBeVisible()
})

test('tous les widgets partagent leur cadre, leur bandeau B et la suppression annulable', async ({
  page,
}, info) => {
  test.setTimeout(90000)
  await page.evaluate((fixture) => {
    const p = { type: 'paragraph', content: [{ type: 'text', text: 'Contenu' }] }
    window.__spike!.editor.commands.setContent({
      type: 'doc',
      content: [
        ...fixture.content.filter((node) => ['bulletList', 'table', 'figure'].includes(node.type)),
        { type: 'orderedList', content: [{ type: 'listItem', content: [p] }] },
        {
          type: 'descriptionList',
          content: [
            {
              type: 'descriptionItem',
              content: [
                { type: 'descriptionTerm', content: [{ type: 'text', text: 'Terme' }] },
                { type: 'descriptionDefinition', content: [{ type: 'text', text: 'Définition' }] },
              ],
            },
          ],
        },
        { type: 'quote', content: [p, { type: 'quoteAttribution' }] },
        { type: 'postscript', content: [{ type: 'text', text: 'Post-scriptum' }] },
        { type: 'attachment', content: [{ type: 'text', text: 'Pièce jointe' }] },
        { type: 'paragraph' },
      ],
    })
  }, fixture)
  const before = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
  await expect(page.locator('[data-editor-widget]')).toHaveCount(8)
  for (const kind of [
    'bulletList',
    'orderedList',
    'descriptionList',
    'table',
    'figure',
    'quote',
    'postscript',
    'attachment',
  ]) {
    const widget = page.locator(`[data-editor-widget="${kind}"]`)
    await expect(widget.locator(':scope > .widget-bar')).toHaveCount(1)
    await expect(widget.locator(':scope > .band-slot')).toHaveCount(1)
    const panel = page.locator('#properties')
    const settings = widget.getByRole('button', { name: /^Paramètres ·/ })
    if (['bulletList', 'orderedList', 'descriptionList', 'figure'].includes(kind)) {
      await widget.getByRole('button', { name: /^Paramètres ·/ }).click()
      await expect(panel).toBeVisible()
      await expect(panel.locator(':scope > .panel-title')).toBeHidden()
      await expect(panel.locator('#remove-block')).toBeHidden()
      await expect(panel.locator('#apply-properties')).toHaveText('Valider')
      await expect(panel.locator(':scope > .context-panel-body')).toHaveCount(1)
      await expect(panel.locator(':scope > .panel-actions')).toHaveCount(1)
      await expect
        .poll(() =>
          widget.evaluate((host) => {
            const rect = host.getBoundingClientRect()
            const style = getComputedStyle(host)
            const band = document.querySelector('#properties')!.getBoundingClientRect()
            const header = host.querySelector('.widget-bar')!.getBoundingClientRect()
            const left = parseFloat(style.borderLeftWidth),
              right = parseFloat(style.borderRightWidth)
            return Math.max(
              Math.abs(band.x - rect.x - left),
              Math.abs(band.y - header.bottom),
              Math.abs(band.width - rect.width + left + right),
            )
          }),
        )
        .toBeLessThan(2)
      expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(
        before,
      )
      await page.keyboard.press('Escape')
      await expect(panel).toBeHidden()
    } else {
      await expect(settings).toBeDisabled()
      await expect(settings).toHaveAttribute('title', /Aucun paramètre supplémentaire/)
      await settings.evaluate((button: HTMLButtonElement) => button.click())
      await expect(panel).toBeHidden()
    }
    await widget.getByRole('button', { name: /^Supprimer ·/ }).click()
    await expect(panel.locator('button:visible')).toHaveText(['Annuler', 'Supprimer'])
    await expect(panel.locator('.context-panel-body')).toBeHidden()
    if (kind === 'quote') await page.screenshot({ path: info.outputPath('quote-frame-remove.png') })
    await expect(page.locator('#apply-properties')).toBeFocused()
    await page.keyboard.press('Enter')
    await expect(widget).toHaveCount(0)
    await page.getByRole('button', { name: 'Annuler', exact: true }).click()
    await expect(widget).toHaveCount(1)
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
    // The direct shortcut uses the same removal operation as the confirmed band.
    await widget.getByRole('button', { name: /^Supprimer ·/ }).click()
    await page.keyboard.press('Escape')
    await widget.locator('.widget-bar > span').click()
    await page.evaluate((kind) => {
      const e = window.__spike!.editor
      e.state.doc.descendants((node, pos) => {
        if (node.type.name === kind) {
          e.commands.setTextSelection(
            pos +
              (kind === 'figure' || ['postscript', 'attachment'].includes(kind)
                ? 1
                : kind === 'quote'
                  ? 2
                  : kind === 'table'
                    ? 4
                    : 3),
          )
          return false
        }
      })
      e.view.focus()
    }, kind)
    await page.keyboard.press('Control+Backspace')
    await expect(widget).toHaveCount(0)
    await page.getByRole('button', { name: 'Annuler', exact: true }).click()
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
  }
})

test('une réponse orthographique tardive ne remplace pas le panneau ouvert ensuite', async ({
  page,
}) => {
  let release!: () => void
  let requested = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON()?.command === 'spelling_suggestions') {
      requested = true
      await new Promise<void>((resolve) => {
        release = resolve
      })
    }
    await route.continue()
  })
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.insertContentAt(1, { type: 'text', text: 'courrrier ' })
    e.commands.setTextSelection(4)
    e.view.focus()
  })
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
  await page.keyboard.press('Control+Enter')
  await expect.poll(() => requested).toBe(true)
  await expect(page.locator('#proofing-band .context-panel-body')).toBeVisible()
  await page.getByRole('button', { name: 'Paramètres · Liste à puces', exact: true }).click()
  await expect(page.locator('#proofing-band')).toHaveCount(0)
  const response = page.waitForResponse(
    (response) => response.request().postDataJSON()?.command === 'spelling_suggestions',
  )
  release()
  await response
  await expect(page.locator('.context-band:visible')).toHaveCount(1)
  await expect(page.locator('#properties')).toBeVisible()
  await expect(page.getByRole('radio', { name: 'Liste à puces', exact: true })).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.locator('.tiptap')).toContainText('courrrier')
})

test('annuler une suppression directe restitue aussi le texte saisi juste avant', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'bulletList') {
        e.commands.setTextSelection(pos + 3)
        return false
      }
    })
    e.view.focus()
  })
  await page.keyboard.type('Ajout ')
  await expect(page.locator('.widget-bulletList li').first()).toContainText('Ajout ')
  const before = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
  await page.keyboard.press('Control+Backspace')
  await expect(page.locator('.widget-bulletList')).toHaveCount(0)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
})

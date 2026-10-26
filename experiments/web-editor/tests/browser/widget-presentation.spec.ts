import { expect, test } from '@playwright/test'

test('citation : en-tête conservé, bandeau dessous et fond de rédaction neutre', async ({
  page,
  request,
}, info) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Préparons la rencontre.' }] },
        {
          type: 'quote',
          content: [
            {
              type: 'paragraph',
              content: [
                {
                  type: 'text',
                  text: 'Un lieu accessible à toutes et à tous, propice aux échanges.',
                },
              ],
            },
            { type: 'quoteAttribution', content: [{ type: 'text', text: 'Projet associatif' }] },
          ],
        },
        { type: 'paragraph' },
      ],
    })
  })
  const widget = page.locator('[data-editor-widget="quote"]')
  const header = widget.locator(':scope > .widget-bar')
  const panel = page.locator('#properties')
  const before = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
  const headerHTML = await header.evaluate((el) => el.innerHTML)
  const toolbarIcon = await page
    .locator('[data-command="block.quote"] svg')
    .evaluate((el) => el.innerHTML)
  expect(await header.locator('.widget-kind svg').evaluate((el) => el.innerHTML)).toBe(toolbarIcon)
  await expect(header.getByRole('img', { name: 'Citation', exact: true })).toBeVisible()
  for (const theme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme: theme })
    await expect
      .poll(() =>
        widget.evaluate((host) => {
          const body = host.querySelector('[data-widget-body]')!
          const bar = host.querySelector('.widget-bar')!
          return {
            body: getComputedStyle(body).backgroundColor,
            root: getComputedStyle(document.documentElement).backgroundColor,
            tinted:
              getComputedStyle(body).backgroundColor !== getComputedStyle(bar).backgroundColor,
          }
        }),
      )
      .toMatchObject({ tinted: true })
    expect(
      await widget
        .locator('[data-widget-body]')
        .evaluate((el) => getComputedStyle(el).backgroundColor),
    ).toBe(await page.locator('html').evaluate((el) => getComputedStyle(el).backgroundColor))
    const collapsedHeader = await header.boundingBox()
    await widget.screenshot({ path: info.outputPath(`quote-${theme}-collapsed.png`) })
    await header.getByRole('button', { name: 'Paramètres · Citation', exact: true }).click()
    await expect(panel).toBeVisible()
    const expandedHeader = await header.boundingBox()
    expect(expandedHeader!.height).toBe(collapsedHeader!.height)
    expect(expandedHeader!.width).toBe(collapsedHeader!.width)
    expect(await header.evaluate((el) => el.innerHTML)).toBe(headerHTML)
    await expect(panel.locator('.panel-title')).toBeHidden()
    await expect(panel.locator('button:visible')).toHaveText(['Annuler', 'Valider'])
    await expect
      .poll(() =>
        page.evaluate(() => {
          const bar = document.querySelector('.widget-quote > .widget-bar')!
          const panel = document.querySelector('#properties')!
          return Math.abs(panel.getBoundingClientRect().top - bar.getBoundingClientRect().bottom)
        }),
      )
      .toBeLessThan(2)
    expect(await header.evaluate((el) => getComputedStyle(el).borderBottomColor)).toBe(
      'rgba(0, 0, 0, 0)',
    )
    await widget.screenshot({ path: info.outputPath(`quote-${theme}-settings.png`) })
    await page.keyboard.press('Enter')
    await expect(panel).toBeHidden()
    await expect(page.locator('.tiptap')).toBeFocused()
    await header.getByRole('button', { name: 'Supprimer · Citation', exact: true }).click()
    await expect(panel.locator('button:visible')).toHaveText(['Annuler', 'Supprimer'])
    await expect(panel.locator('.context-panel-body')).toBeHidden()
    expect(await header.evaluate((el) => el.innerHTML)).toBe(headerHTML)
    await widget.screenshot({ path: info.outputPath(`quote-${theme}-remove.png`) })
    await page.keyboard.press('Shift+Tab')
    await expect(page.locator('#cancel-properties')).toBeFocused()
    await page.keyboard.press('Enter')
    await expect(panel).toBeHidden()
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
  }
})

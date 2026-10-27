import { expect, test } from '@playwright/test'

test('citation : paramètres indisponibles, filet intérieur et attribution à gauche', async ({
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
            {
              type: 'paragraph',
              content: [{ type: 'text', text: 'Une rencontre ouverte aux échanges.' }],
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
    const settings = header.getByRole('button', { name: 'Paramètres · Citation', exact: true })
    await expect(settings).toBeDisabled()
    await expect(settings).toHaveAttribute('title', /Aucun paramètre supplémentaire/)
    const geometry = await widget.evaluate((host) => {
      const body = host.querySelector('blockquote')!,
        p = body.querySelector('p')!,
        cite = body.querySelector('cite')!
      const style = getComputedStyle(body),
        bounds = body.getBoundingClientRect(),
        attribution = cite.getBoundingClientRect()
      return {
        outer: getComputedStyle(host).borderLeftWidth,
        text: getComputedStyle(p).borderLeftWidth,
        source: getComputedStyle(cite).borderLeftWidth,
        align: getComputedStyle(cite).textAlign,
        rule: getComputedStyle(cite).borderTopWidth,
        widthGap: Math.abs(
          attribution.width -
            bounds.width +
            parseFloat(style.paddingLeft) +
            parseFloat(style.paddingRight),
        ),
        leftGap: Math.abs(attribution.left - bounds.left - parseFloat(style.paddingLeft)),
      }
    })
    expect(geometry).toMatchObject({
      outer: '1px',
      text: '5px',
      source: '0px',
      align: 'left',
      rule: '1px',
    })
    expect(geometry.widthGap).toBeLessThan(2)
    expect(geometry.leftGap).toBeLessThan(2)
    await expect(widget.locator('.paragraph-marker')).toHaveCSS('border-left-width', '5px')
    await page.evaluate(() => {
      const e = window.__spike!.editor
      e.state.doc.descendants((node, pos) => {
        if (node.type.name === 'quote') {
          e.commands.setTextSelection(pos + 2)
          return false
        }
      })
      e.view.focus()
    })
    for (const key of ['Control+Enter', 'Meta+Enter']) {
      await page.keyboard.press(key)
      await expect(panel).toBeHidden()
      expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(
        before,
      )
    }
    await page.keyboard.press('Control+Shift+P')
    await page
      .getByRole('textbox', { name: 'Rechercher une commande', exact: true })
      .fill('Paramètres')
    await expect(
      page.getByRole('button', { name: 'Paramètres · Mod+Enter', exact: true }),
    ).toBeDisabled()
    await page.keyboard.press('Escape')
    await expect(page.locator('.tiptap')).toBeFocused()
    // Editing the direct attribution remains available without a settings panel.
    await page.keyboard.press('Tab')
    await expect
      .poll(() =>
        page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name),
      )
      .toBe('quoteAttribution')
    await header.getByRole('button', { name: 'Supprimer · Citation', exact: true }).click()
    await expect(panel.locator('button:visible')).toHaveText(['Annuler', 'Supprimer'])
    await expect(panel.locator('.context-panel-body')).toBeHidden()
    expect(await header.evaluate((el) => el.innerHTML)).toBe(headerHTML)
    await page.keyboard.press('Shift+Tab')
    await expect(page.locator('#cancel-properties')).toBeFocused()
    await page.keyboard.press('Enter')
    await expect(panel).toBeHidden()
    expect(await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))).toBe(before)
  }
})

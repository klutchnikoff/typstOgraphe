import { expect, test } from '@playwright/test'

test.beforeEach(async ({ request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
})
for (const profile of ['rich-text', 'inline-text']) {
  test(`${profile} : une nouvelle zone hors de l’en-tête hérite du clavier, des liens et de l’historique`, async ({
    page,
  }) => {
    await page.goto('/?client=main')
    await page.waitForFunction(() => window.__spike?.ready)
    const before = await page.evaluate(() => {
      const editor = window.__spike!.editor
      editor.commands.insertContent('Le corps doit rester intact. ')
      return editor.getJSON()
    })
    await page.evaluate(
      async ({ path, profile }) => {
        const { createRichZone } = await import(path)
        const mount = document.createElement('div')
        mount.style.cssText =
          'position:fixed;top:100px;left:20px;width:450px;background:white;z-index:9999'
        document.body.append(mount)
        let linkCalls = 0
        const zone = createRichZone({
          profile,
          element: mount,
          label: 'Zone indépendante',
          language: 'fr',
          content: '<p>Texte</p>',
          openLink: (editor: import('@tiptap/core').Editor) => {
            linkCalls++
            editor.commands.setLink({ href: 'https://example.org' })
            mount.dataset.links = String(linkCalls)
          },
        })
        if (!zone.isInitialized)
          await new Promise<void>((resolve) => zone.once('create', () => resolve()))
        zone.commands.focus('end')
      },
      { path: '/src/rich-zone.ts', profile },
    )
    const zone = page.getByRole('textbox', { name: 'Zone indépendante', exact: true })
    await expect(zone).toBeFocused()
    await zone.press('ControlOrMeta+b')
    await page.keyboard.type(' enrichi')
    await expect(zone.locator('strong')).toHaveText(' enrichi')
    await zone.press('ControlOrMeta+z')
    await expect(zone).toHaveText('Texte')
    await zone.press('ControlOrMeta+Shift+z')
    await expect(zone).toHaveText('Texte enrichi')
    await zone.press('ControlOrMeta+a')
    await zone.press('ControlOrMeta+k')
    await expect(zone.locator('a')).toHaveAttribute('href', 'https://example.org')
    await zone.locator('a').click()
    await zone.press('ControlOrMeta+Enter')
    await expect(page.locator('[data-links="2"]')).toBeVisible()
    await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    await expect(zone).toHaveAttribute('spellcheck', 'false')
    await expect(zone).toHaveAttribute('lang', 'fr')
  })
}

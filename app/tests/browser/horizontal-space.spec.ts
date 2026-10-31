import { expect, test } from '@playwright/test'
import { confirmNewDocument } from './document-lifecycle'

test.beforeEach(async ({ request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
})
for (const profile of ['body', 'title', 'abstract']) {
  test(`les frappes d’espace alternent sans cumul dans ${profile}`, async ({ page }) => {
    await page.goto('/?client=main')
    await page.waitForFunction(() => window.__spike?.ready)
    if (profile !== 'body') {
      await page.getByRole('button', { name: 'Fichier', exact: true }).click()
      await page.locator('[data-command="document.new"]').click()
      await page.locator('dialog select').first().selectOption('article')
      await confirmNewDocument(page)
      await page.locator(`#metadata-tab-${profile === 'title' ? 'titles' : 'abstracts'}`).click()
    }
    const zone = page.getByRole('textbox', {
      name: profile === 'body' ? 'Corps du document' : profile === 'title' ? 'Titre' : 'Résumé',
      exact: true,
    })
    await zone.fill('Deux')
    for (const value of [' ', '\u00a0', ' ', '\u00a0']) {
      await zone.press('Space')
      await expect.poll(() => zone.textContent()).toBe(`Deux${value}`)
      await expect(zone.locator('.nonbreaking-space')).toHaveCount(value === ' ' ? 0 : 1)
    }
    const marker = zone.locator('.nonbreaking-space')
    const css = await marker.evaluate((el) => ({
      content: getComputedStyle(el, '::before').content,
      color: getComputedStyle(el, '::before').color,
      muted: getComputedStyle(el).getPropertyValue('--muted').trim(),
    }))
    expect(css.content).toContain('~')
    // A pseudo-element does not reserve its own width. The actual NBSP must
    // occupy the tilde's natural advance in every rich-zone profile.
    const advance = await marker.evaluate(async (el) => {
      await document.fonts.ready
      const measure = (text: string) => {
        const range = document.createRange()
        if (text === '\u00a0') {
          range.selectNodeContents(el)
          return range.getBoundingClientRect().width
        }
        const probe = document.createElement('span')
        const style = getComputedStyle(el, '::before')
        probe.textContent = text
        probe.style.fontFamily = style.fontFamily
        probe.style.fontSize = style.fontSize
        probe.style.fontWeight = style.fontWeight
        probe.style.fontStyle = style.fontStyle
        probe.style.letterSpacing = style.letterSpacing
        probe.style.position = 'absolute'
        probe.style.visibility = 'hidden'
        document.body.append(probe)
        const width = probe.getBoundingClientRect().width
        probe.remove()
        return width
      }
      return { space: measure('\u00a0'), tilde: measure('~') }
    })
    expect(advance.space).toBeGreaterThan(0)
    expect(Math.abs(advance.space - advance.tilde)).toBeLessThan(0.5)
    await page.evaluate(async () => {
      await window.__spike!.provider.clearHistory()
    })
    await page.keyboard.type('mots')
    await expect.poll(() => zone.textContent()).toBe('Deux\u00a0mots')
    await zone.press('ControlOrMeta+z')
    await expect.poll(() => zone.textContent()).toBe('Deux\u00a0')
    await zone.press('ControlOrMeta+Shift+z')
    await expect.poll(() => zone.textContent()).toBe('Deux\u00a0mots')
    await expect(zone).toBeFocused()
    if (profile === 'body') {
      await page.evaluate(() =>
        window.__spike!.editor.commands.setTextSelection({ from: 5, to: 6 }),
      )
      expect(await page.evaluate(() => document.getSelection()?.toString())).toBe('\u00a0')
      await zone.press('ArrowRight')
      await expect
        .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.from))
        .toBe(6)
    }
  })
}

import { expect, test } from '@playwright/test'

function contrast(first: string, second: string) {
  const luminance = (color: string) => {
    const channels = color
      .match(/[\d.]+/g)!
      .slice(0, 3)
      .map(Number)
    return channels
      .map((value) => {
        const channel = value / 255
        return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
      })
      .reduce((sum, value, i) => sum + value * [0.2126, 0.7152, 0.0722][i], 0)
  }
  const a = luminance(first),
    b = luminance(second)
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

test('les couleurs sémantiques restent lisibles et cohérentes entre thème système et explicite', async ({
  page,
  request,
}) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(() => {
    window.__spike!.editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'Gras', marks: [{ type: 'bold' }] },
            { type: 'text', text: 'Italique', marks: [{ type: 'italic' }] },
            { type: 'footnote', attrs: { text: 'Précision' } },
            { type: 'citation', attrs: { items: [{ key: 'dupont2020' }] } },
          ],
        },
        { type: 'paragraph' },
      ],
    })
    window.__spike!.editor.commands.setTextSelection(1)
  })
  for (const theme of ['light', 'dark'] as const) {
    let system: unknown
    for (const appearance of ['system', theme]) {
      // An explicit appearance must also win over the opposite OS setting.
      await page.emulateMedia({
        colorScheme: appearance === 'system' ? theme : theme === 'light' ? 'dark' : 'light',
      })
      await page.evaluate((value) => {
        document.documentElement.dataset.appearance = value
      }, appearance)
      const palette = await page.evaluate(() => {
        const color = (selector: string) =>
          getComputedStyle(document.querySelector(selector)!).color
        const badge = (kind: string) => {
          const element = document.querySelector<HTMLElement>(
            `.inline-object[data-kind="${kind}"]`,
          )!
          const style = getComputedStyle(element.querySelector('.inline-object-label')!)
          return { ink: style.color, surface: style.backgroundColor, border: style.borderTopColor }
        }
        return {
          strong: color('#editor strong'),
          emphasis: color('#editor em'),
          background: getComputedStyle(document.documentElement).getPropertyValue('--panel').trim(),
          note: badge('footnote'),
          citation: badge('citation'),
        }
      })
      // Resolve the editor surface to RGB just like the rendered text colors.
      const background = await page.evaluate((value) => {
        const sample = document.createElement('span')
        sample.style.color = value
        document.body.append(sample)
        const result = getComputedStyle(sample).color
        sample.remove()
        return result
      }, palette.background)
      expect(contrast(palette.strong, background)).toBeGreaterThanOrEqual(4.5)
      expect(contrast(palette.emphasis, background)).toBeGreaterThanOrEqual(4.5)
      expect(palette.note.ink).not.toBe(palette.citation.ink)
      expect(palette.note.surface).not.toBe(palette.citation.surface)
      for (const badge of [palette.note, palette.citation]) {
        expect(contrast(badge.ink, badge.surface)).toBeGreaterThanOrEqual(4.5)
        expect(contrast(background, badge.ink)).toBeGreaterThanOrEqual(4.5)
      }
      if (appearance === 'system') system = palette
      else expect(palette).toEqual(system)
    }
  }
})

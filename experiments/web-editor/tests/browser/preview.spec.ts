import { expect, test } from '@playwright/test'

declare global {
  interface Window {
    __previewProbe: {
      hold: boolean
      pending: (() => void)[]
      additions: { complete: boolean; width: number; height: number }[]
    }
  }
}

test('les pages restent affichées jusqu’au décodage de tout le nouvel aperçu', async ({
  page,
  request,
}, info) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.addInitScript(() => {
    const probe = (window.__previewProbe = {
      hold: false,
      pending: [] as (() => void)[],
      additions: [] as { complete: boolean; width: number; height: number }[],
    })
    const decode = HTMLImageElement.prototype.decode
    HTMLImageElement.prototype.decode = function () {
      const decoded = decode.call(this)
      if (!probe.hold || !this.alt.startsWith('Page ')) return decoded
      return Promise.all([
        decoded,
        new Promise<void>((resolve) => probe.pending.push(resolve)),
      ]).then(() => {})
    }
    const observer = new MutationObserver((records) => {
      for (const record of records)
        for (const added of record.addedNodes) {
          if (!(added instanceof HTMLElement)) continue
          for (const img of added.querySelectorAll('img')) {
            if (img.closest('#pages'))
              probe.additions.push({
                complete: img.complete,
                width: img.naturalWidth,
                height: img.naturalHeight,
              })
          }
        }
    })
    const observe = () =>
      observer.observe(document.documentElement, { childList: true, subtree: true })
    if (document.documentElement) observe()
    else document.addEventListener('DOMContentLoaded', observe, { once: true })
  })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('#pages img')).toHaveCount(2)
  await expect(page.locator('#render-status')).toHaveText(/^2 pages ·/)
  const scrollBefore = await page.locator('#pages').evaluate((element) => {
    element.scrollTop = 180
    return element.scrollTop
  })
  const before = await page.locator('#pages').innerHTML()
  await page.evaluate(() => {
    window.__previewProbe.hold = true
    window.__previewProbe.additions = []
    const e = window.__spike!.editor
    e.commands.insertContentAt(1, 'Aperçu actualisé : ')
    e.commands.blur()
  })
  await expect.poll(() => page.evaluate(() => window.__previewProbe.pending.length)).toBe(2)
  expect(await page.locator('#pages').innerHTML()).toBe(before)
  await expect(page.locator('#render-status')).toHaveText('Mise à jour…')
  await page.screenshot({ path: info.outputPath('old-preview-while-decoding.png') })
  await page.evaluate(() => window.__previewProbe.pending.shift()!())
  expect(await page.locator('#pages').innerHTML()).toBe(before)
  await page.evaluate(() => window.__previewProbe.pending.shift()!())
  await expect(page.locator('#render-status')).toHaveText(/^2 pages ·/)
  expect(await page.locator('#pages').innerHTML()).not.toBe(before)
  expect(await page.locator('#pages').evaluate((element) => element.scrollTop)).toBe(scrollBefore)
  const added = await page.evaluate(() => window.__previewProbe.additions)
  expect(added).toHaveLength(2)
  expect(added.every((img) => img.complete && img.width > 0 && img.height > 0)).toBe(true)
  await page.screenshot({ path: info.outputPath('decoded-preview.png') })
})

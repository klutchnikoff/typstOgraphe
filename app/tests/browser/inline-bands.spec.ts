import { expect, test, type Page } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

async function lineGeometry(page: Page) {
  return page.evaluate(() => {
    const p = document.querySelector('.tiptap p')!
    const walker = document.createTreeWalker(p, NodeFilter.SHOW_TEXT)
    const rects: DOMRect[] = []
    while (walker.nextNode()) {
      const text = walker.currentNode
      if (!text.textContent?.trim() || text.parentElement?.closest('[contenteditable=false]'))
        continue
      const range = document.createRange()
      range.selectNodeContents(text)
      rects.push(...range.getClientRects())
    }
    const band = document.querySelector('#properties')!.getBoundingClientRect()
    return {
      tops: rects.map((rect) => rect.top),
      lineSpread:
        Math.max(...rects.map((rect) => rect.top)) - Math.min(...rects.map((rect) => rect.top)),
      bandBottom: band.bottom,
      firstTop: rects[0].top,
      document: window.__spike!.editor.getJSON(),
    }
  })
}

async function glyphs(page: Page) {
  return page.evaluate(() => {
    const e = window.__spike!.editor,
      origin = e.view.dom.getBoundingClientRect()
    const positions: { x: number; y: number }[] = []
    e.state.doc.descendants((node, pos) => {
      if (node.isText)
        for (let index = 0; index < node.nodeSize; index++) {
          const rect = e.view.coordsAtPos(pos + index)
          positions.push({ x: rect.left - origin.left, y: rect.top - origin.top })
        }
    })
    return positions
  })
}

async function unchangedWrapping(page: Page, before: Awaited<ReturnType<typeof glyphs>>) {
  const after = await glyphs(page)
  expect(after).toHaveLength(before.length)
  // Every character keeps its horizontal position: opening a band must not
  // introduce a new visual break, including inside a word or a styled run.
  expect(Math.max(...after.map((rect, index) => Math.abs(rect.x - before[index].x)))).toBeLessThan(
    1,
  )
}

async function aboveTarget(page: Page, band: string, selector: string) {
  await expect
    .poll(async () => {
      const panel = await page.locator(band).boundingBox(),
        target = await page.locator(selector).boundingBox()
      return target!.y - panel!.y - panel!.height
    })
    .toBeGreaterThan(0)
  await expect
    .poll(async () => {
      const panel = await page.locator(band).boundingBox(),
        target = await page.locator(selector).boundingBox()
      return target!.y - panel!.y - panel!.height
    })
    .toBeLessThan(35)
}

test('un bandeau de note préserve toute la ligne « Test des notes »', async ({ page }, info) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent('<p>Test des notes</p>')
    e.commands.setTextSelection(5)
  })
  await page.locator('[data-command="insert.footnote"]').click()
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
  const metrics = await lineGeometry(page)
  await info.attach('geometry', { body: JSON.stringify(metrics), contentType: 'application/json' })
  expect(metrics.lineSpread).toBeLessThan(2)
  expect(metrics.bandBottom).toBeLessThan(metrics.firstTop)
})

for (const context of ['prose', 'list', 'quote'] as const) {
  test(`placement partagé : note, lien surélevé et orthographe dans ${context}`, async ({
    page,
  }, info) => {
    await page.addStyleTag({
      content:
        '.tiptap a { font-size: .65em; vertical-align: super; } .tiptap strong { font-size: 1.15em; }',
    })
    await page.evaluate((context) => {
      const paragraph = {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'Test', marks: [{ type: 'bold' }] },
          { type: 'footnote', attrs: { text: 'Une note' } },
          { type: 'text', text: ' des ' },
          {
            type: 'text',
            text: 'liens',
            marks: [{ type: 'link', attrs: { href: 'https://example.org' } }],
          },
          { type: 'text', text: ' dans ce courrrier.' },
        ],
      }
      window.__spike!.editor.commands.setContent({
        type: 'doc',
        content:
          context === 'list'
            ? [{ type: 'bulletList', content: [{ type: 'listItem', content: [paragraph] }] }]
            : context === 'quote'
              ? [{ type: 'quote', content: [paragraph, { type: 'quoteAttribution' }] }]
              : [paragraph],
      })
    }, context)
    await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
      timeout: 15000,
    })
    const before = await glyphs(page)
    const document = await page.evaluate(() => window.__spike!.editor.getJSON())
    for (const target of ['note', 'link', 'spelling']) {
      const selector =
        target === 'note'
          ? '.footnote-ref .inline-object-label'
          : target === 'link'
            ? '.tiptap a'
            : '.spelling-error[data-word="courrrier"]'
      if (target === 'note') await page.locator(selector).click()
      else {
        await page.evaluate((target) => {
          const e = window.__spike!.editor
          let at = 1
          e.state.doc.descendants((node, pos) => {
            const word = target === 'link' ? 'liens' : 'courrrier'
            if (node.isText && node.text!.includes(word)) at = pos + node.text!.indexOf(word) + 2
          })
          e.commands.setTextSelection(at)
          e.view.focus()
        }, target)
        await page.keyboard.press('Control+Enter')
      }
      const band = target === 'spelling' ? '#proofing-band' : '#properties'
      await expect(page.locator(band)).toBeVisible()
      await unchangedWrapping(page, before)
      await aboveTarget(page, band, selector)
      expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(document)
      await page.keyboard.press('Escape')
      await expect(page.locator(band)).toBeHidden()
      await unchangedWrapping(page, before)
    }
  })
}

test('ligne ciblée après un retour simple ou un repli automatique, puis redimensionnement', async ({
  page,
}, info) => {
  for (const softBreak of [true, false]) {
    await page.evaluate((softBreak) => {
      const prefix = softBreak
        ? 'Une première ligne.'
        : 'Une première phrase à conserver avant le passage ciblé. '.repeat(4)
      window.__spike!.editor.commands.setContent({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [
              { type: 'text', text: prefix },
              ...(softBreak ? [{ type: 'softBreak' }] : []),
              { type: 'text', text: 'Test' },
              { type: 'footnote', attrs: { text: 'Note existante' } },
              {
                type: 'text',
                text: ' des notes au milieu de la phrase pour vérifier sa disposition.',
              },
            ],
          },
        ],
      })
    }, softBreak)
    for (const width of [1280, 900, 700]) {
      await page.setViewportSize({ width, height: 900 })
      const before = await glyphs(page)
      await page.locator('.footnote-ref .inline-object-label').click()
      await expect(page.locator('#properties')).toBeVisible()
      await unchangedWrapping(page, before)
      await aboveTarget(page, '#properties', '.footnote-ref .inline-object-label')
      await page.setViewportSize({ width: width - 30, height: 900 })
      await aboveTarget(page, '#properties', '.footnote-ref .inline-object-label')
      const expanded = await glyphs(page)
      await page.keyboard.press('Escape')
      await unchangedWrapping(page, expanded)
    }
  }
})

test('le secours CJK reste inutilisé pour le texte latin même sans la police système choisie', async ({
  page,
}) => {
  const cjkRequests: string[] = []
  page.on('request', (request) => {
    if (request.url().includes('NotoSansCJKjp')) cjkRequests.push(request.url())
  })
  await page.evaluate(async () => {
    document.documentElement.style.setProperty('--editor-font', 'Police volontairement absente')
    window.__spike!.editor.commands.setContent(
      '<p>Un courrier français, lisible sans police CJK.</p>',
    )
    document.querySelector('.tiptap')!.getBoundingClientRect()
    await document.fonts.ready
  })
  const statuses = () =>
    page.evaluate(() =>
      [...document.fonts]
        .filter((font) => font.family.includes('Noto CJK'))
        .map((font) => font.status),
    )
  expect(await statuses()).not.toContain('loaded')
  expect(cjkRequests).toHaveLength(0)
  await page.evaluate(() => {
    window.__spike!.editor.commands.insertContent('Japonais : にほん。Chinois : 中文。')
  })
  await expect.poll(() => cjkRequests.length).toBeGreaterThan(0)
  await page.evaluate(() => document.fonts.ready.then(() => undefined))
  expect(await statuses()).toContain('loaded')
})

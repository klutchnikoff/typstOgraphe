import { expect, test, type Page } from '@playwright/test'

async function contents(page: Page) {
  return page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
}
async function ready(page: Page) {
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await expect(page.locator('#pages .preview-page').first()).toBeVisible()
}
test.beforeEach(async ({ page, request }) => {
  const response = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await response.json()).error).toBeUndefined()
  await ready(page)
})
async function clear(page: Page) {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.clearContent()
    e.commands.focus('start')
  })
  await expect(page.locator('.tiptap')).toBeFocused()
}
test('retours limités, marque de paragraphe et raccourci de liste après un retour simple', async ({
  page,
}) => {
  await clear(page)
  await page.keyboard.type('Avant')
  await page.keyboard.press('Enter')
  await page.keyboard.type('- Un')
  await expect(page.locator('.tiptap > p').first()).toHaveText('Avant')
  await expect(page.locator('.widget-bulletList li')).toHaveText('Un')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Enter')
  await page.keyboard.type('Après')
  await expect(page.locator('.tiptap > p').last()).toHaveText('Après')
  await clear(page)
  await page.keyboard.type('Premier')
  await page.keyboard.press('Enter')
  await page.keyboard.type('suite')
  await expect(page.locator('.tiptap p')).toHaveCount(1)
  await expect(page.locator('.paragraph-marker')).toHaveCount(0)
  for (let i = 0; i < 5; i++) await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap p')).toHaveCount(2)
  await expect(page.locator('.paragraph-marker')).toHaveText('¶')
  await page.keyboard.type('Second')
  expect(await contents(page)).not.toContain('¶')
  await clear(page)
  await page.keyboard.type('- Premier')
  await expect(page.locator('.widget-bulletList li')).toHaveText('Premier')
})
test('insertion des widgets au curseur sans absorber le texte, citation sans paragraphes vides', async ({
  page,
}) => {
  for (const id of ['block.bullet', 'block.quote']) {
    await page.evaluate(() => {
      const e = window.__spike!.editor
      e.commands.setContent('<p>AvantAprès</p>')
      e.commands.setTextSelection(6)
      e.commands.focus()
    })
    if (id === 'block.bullet')
      await page.getByRole('button', { name: 'Listes', exact: true }).click()
    await page.locator(`[data-command="${id}"]`).click()
    const widget = page.locator(id === 'block.quote' ? '.widget-quote' : '.widget-bulletList')
    await expect(widget).toBeVisible()
    await expect(widget.locator('p')).toHaveText('')
    await expect(page.locator('.tiptap > p')).toHaveText(['Avant', '', 'Après'])
    await expect(page.locator('.tiptap')).toBeFocused()
    await page.keyboard.type('Contenu')
    await expect(widget.locator('p')).toHaveText('Contenu')
    if (id === 'block.quote') {
      await page.keyboard.press('Enter')
      await page.keyboard.type('suite')
      await expect(widget.locator('p')).toHaveCount(1)
      for (let i = 0; i < 4; i++) await page.keyboard.press('Enter')
      await expect(widget.locator('p')).toHaveCount(2)
      await expect(widget.locator('.paragraph-marker')).toHaveText('¶')
    }
  }
})
test('Retour arrière dans une liste rejoint le texte précédent sans désassembler le widget', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Avant' }] },
        {
          type: 'bulletList',
          content: ['Un', 'Deux', ''].map((text) => ({
            type: 'listItem',
            content: [{ type: 'paragraph', content: text ? [{ type: 'text', text }] : [] }],
          })),
        },
      ],
    })
    let pos = 0
    e.state.doc.descendants((node, offset) => {
      if (node.type.name === 'paragraph' && !node.content.size) pos = offset + 1
    })
    e.commands.setTextSelection(pos)
    e.commands.focus()
  })
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.keyboard.press('Backspace')
  await expect(page.locator('li')).toHaveCount(2)
  await page.keyboard.type(' suite')
  await expect(page.locator('li').last()).toHaveText('Deux suite')
  for (
    let n = await page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset);
    n > 0;
    n--
  )
    await page.keyboard.press('ArrowLeft')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset))
    .toBe(0)
  await page.keyboard.press('Backspace')
  await expect(page.locator('li')).toHaveCount(1)
  await expect(page.locator('li')).toHaveText('UnDeux suite')
  for (
    let n = await page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset);
    n > 0;
    n--
  )
    await page.keyboard.press('ArrowLeft')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parentOffset))
    .toBe(0)
  await page.keyboard.press('Backspace')
  await expect(page.locator('.widget-bulletList')).toHaveCount(1)
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.depth))
    .toBe(1)
  await page.keyboard.type(' précédent')
  await expect(page.locator('.tiptap > p')).toHaveText('Avant précédent')
})
test('variante B : paramètres de liste intégrés, brouillon, validation, annulation et suppression', async ({
  page,
}, info) => {
  const before = await contents(page)
  const button = page.getByRole('button', { name: 'Paramètres · Liste à puces', exact: true })
  await button.click()
  const form = page.locator('#properties')
  await expect(form).toBeVisible()
  await expect(page.getByRole('radio', { name: 'Liste à puces', exact: true })).toBeFocused()
  const widget = await page.locator('.widget-bulletList').boundingBox(),
    band = await form.boundingBox()
  expect(Math.abs(band!.x - widget!.x - 1)).toBeLessThan(2)
  const header = await page.locator('.widget-bulletList > .widget-bar').boundingBox()
  expect(Math.abs(band!.y - header!.y - header!.height)).toBeLessThan(2)
  expect(Math.abs(band!.width - widget!.width + 2)).toBeLessThan(2)
  await page.keyboard.press('ArrowRight')
  await expect(page.getByRole('radio', { name: 'Liste numérotée', exact: true })).toHaveAttribute(
    'aria-checked',
    'true',
  )
  await page.getByLabel('Numéro initial', { exact: true }).fill('3')
  expect(await contents(page)).toBe(before)
  await page.keyboard.press('Escape')
  await expect(form).toBeHidden()
  expect(await contents(page)).toBe(before)
  await button.click()
  await expect(page.getByRole('radio', { name: 'Liste à puces', exact: true })).toHaveAttribute(
    'aria-checked',
    'true',
  )
  await page.getByRole('radio', { name: 'Liste numérotée', exact: true }).click()
  await page.getByLabel('Numéro initial', { exact: true }).fill('3')
  await page.keyboard.press('Enter')
  await expect(form).toBeHidden()
  await expect(page.locator('.widget-orderedList ol')).toHaveAttribute('start', '3')
  await page.getByRole('button', { name: 'Supprimer · Liste numérotée', exact: true }).click()
  await expect(page.locator('#apply-properties')).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.locator('.widget-orderedList')).toHaveCount(1)
})
test('image : bandeau intégré, focus et modification annulable du texte alternatif', async ({
  page,
}, info) => {
  const before = await contents(page)
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  const field = page.getByLabel('Texte alternatif', { exact: true })
  await expect(field).toBeFocused()
  const widget = await page.locator('figure').boundingBox(),
    band = await page.locator('#properties').boundingBox()
  const header = await page.locator('figure > .widget-bar').boundingBox()
  expect(Math.abs(band!.y - header!.y - header!.height)).toBeLessThan(2)
  await field.fill('Description provisoire')
  expect(await contents(page)).toBe(before)
  await page.keyboard.press('Escape')
  expect(await contents(page)).toBe(before)
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await field.fill('Description validée')
  for (const id of ['replace-image', 'cancel-properties', 'apply-properties']) {
    await page.keyboard.press('Tab')
    await expect(page.locator(`#${id}`)).toBeFocused()
  }
  await page.keyboard.press('Enter')
  await expect(page.locator('figure img')).toHaveAttribute('alt', 'Description validée')
  await expect(page.locator('.tiptap')).toBeFocused()
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  expect(await contents(page)).toBe(before)
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await expect(page.locator('#properties > .panel-title')).toBeHidden()
  await expect(page.locator('#remove-block')).toBeHidden()
  await expect(page.locator('#apply-properties')).toHaveText('Valider')
  await expect(page.locator('#properties .panel-actions')).toBeInViewport()
  await expect(page.locator('#render-status')).toHaveText(/^\d+ pages? ·/)
})
test('orthographe interligne : pointe vers le mot, largeur du bloc et réponse au redimensionnement', async ({
  page,
}, info) => {
  for (const inBlock of [false, true, 'quote'] as const) {
    await page.evaluate((inBlock) => {
      const e = window.__spike!.editor
      const p = {
        type: 'paragraph',
        content: [
          {
            type: 'text',
            text: 'Nous vous remercions pour votre retour. Merci de confirmer la réception de ce courrrier avant vendredi pour préparer la rencontre.',
          },
        ],
      }
      e.commands.setContent({
        type: 'doc',
        content:
          inBlock === 'quote'
            ? [{ type: 'quote', content: [p, { type: 'quoteAttribution' }] }]
            : inBlock
              ? [{ type: 'bulletList', content: [{ type: 'listItem', content: [p] }] }]
              : [p],
      })
      let pos = 1
      e.state.doc.descendants((node, offset) => {
        if (node.isText && node.text!.includes('courrrier'))
          pos = offset + node.text!.indexOf('courrrier') + 3
      })
      e.commands.setTextSelection(pos)
      e.commands.focus()
    }, inBlock)
    await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
      timeout: 15000,
    })
    await expect(page.locator('.spelling-error[data-word="courrrier"]')).toHaveCount(1)
    await expect(page.locator('.tiptap')).toHaveAttribute('spellcheck', 'false')
    const before = await contents(page)
    await page.keyboard.press('Control+Enter')
    const band = page.locator('#proofing-band')
    await expect(band).toBeVisible()
    await expect(band.locator('[data-default]')).toBeVisible()
    for (const width of [1280, 900]) {
      await page.setViewportSize({ width, height: 900 })
      await expect
        .poll(async () => {
          const word = await page.locator('.spelling-error[data-word="courrrier"]').boundingBox()
          const rect = await band.boundingBox()
          return Math.abs(word!.y - (rect!.y + rect!.height))
        })
        .toBeLessThan(35)
      if (inBlock)
        await expect
          .poll(() =>
            page.evaluate(() => {
              const rect = document.querySelector('#proofing-band')!.getBoundingClientRect()
              const host = document.querySelector('[data-editor-widget]')!.getBoundingClientRect()
              return Math.abs(rect.width - host.width + 2)
            }),
          )
          .toBeLessThan(2)
      await expect
        .poll(() =>
          page.evaluate(() => {
            const band = document.querySelector('#proofing-band')!
            const rect = band.getBoundingClientRect()
            const arrow = parseFloat(getComputedStyle(band, '::after').left)
            const word = document
              .querySelector('.spelling-error[data-word="courrrier"]')!
              .getBoundingClientRect()
            return Math.abs(rect.x + arrow - word.x - word.width / 2)
          }),
        )
        .toBeLessThan(4)
    }
    expect(await contents(page)).toBe(before)
    await page.keyboard.press('Escape')
    await expect(band).toHaveCount(0)
    expect(await contents(page)).toBe(before)
    await page.keyboard.press('Control+Enter')
    await expect(page.locator('#proofing-band [data-default]')).toBeVisible()
    await expect(page.locator('#proofing-band')).toBeFocused()
    await page.keyboard.press('Enter')
    await expect(page.locator('.tiptap')).not.toContainText('courrrier')
    await expect(page.locator('.tiptap')).toBeFocused()
    await page.setViewportSize({ width: 1280, height: 900 })
  }
})

test('un seul brouillon, reprise au raccourci et refus des paramètres périmés', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  const alt = page.getByLabel('Texte alternatif', { exact: true })
  await alt.fill('Brouillon conservé')
  await page.evaluate(() => window.__spike!.editor.view.focus())
  await page.keyboard.press('Control+Enter')
  await expect(alt).toHaveValue('Brouillon conservé')
  await expect(page.locator('#properties')).toBeFocused()
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.insertContentAt(1, 'Ajout avant le bloc. ')
  })
  await page.keyboard.press('Enter')
  await expect(page.locator('figure img')).toHaveAttribute('alt', 'Brouillon conservé')
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await alt.fill('Ne doit pas écraser la modification distante')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'figure')
        e.view.dispatch(
          e.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, alt: 'Modification distante' }),
        )
    })
  })
  await page.keyboard.press('Enter')
  await expect(page.locator('#property-error')).toContainText('modifié ailleurs')
  await expect(page.locator('figure img')).toHaveAttribute('alt', 'Modification distante')
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await alt.fill('Brouillon abandonné')
  await page.getByRole('button', { name: 'Paramètres · Liste à puces', exact: true }).click()
  await expect(page.locator('#properties')).toHaveCount(1)
  await expect(alt).toBeHidden()
  await page.keyboard.press('Escape')
  await expect(page.locator('figure img')).toHaveAttribute('alt', 'Modification distante')
})

test('liens et suppression de tableau utilisent le même placement et le même contrat clavier', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.insertContentAt(1, {
      type: 'text',
      text: 'Lien de test ',
      marks: [{ type: 'link', attrs: { href: 'https://example.org' } }],
    })
    e.commands.setTextSelection(5)
    e.view.focus()
  })
  await page.keyboard.press('Control+Enter')
  await expect(page.getByLabel('Adresse du lien', { exact: true })).toBeFocused()
  await expect(page.locator('#properties')).toHaveClass(/inline-context-band/)
  await page.getByLabel('Adresse du lien', { exact: true }).fill('https://example.com')
  await page.keyboard.press('Enter')
  await expect(page.locator('.tiptap a')).toHaveAttribute('href', 'https://example.com')
  await expect(
    page.getByRole('button', { name: 'Paramètres · Tableau', exact: true }),
  ).toBeDisabled()
  await page.locator('td').first().click()
  await page.getByRole('button', { name: 'Supprimer le tableau', exact: true }).click()
  const form = page.locator('#properties')
  await expect(form).toBeVisible()
  const host = await page.locator('.widget-table').boundingBox(),
    rect = await form.boundingBox()
  const header = await page.locator('.widget-table > .widget-bar').boundingBox()
  expect(Math.abs(rect!.y - header!.y - header!.height)).toBeLessThan(2)
  await page.keyboard.press('Escape')
  await expect(form).toBeHidden()
})

test('Retour arrière respecte la frontière terme et définition', async ({ page }) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
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
      ],
    })
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'descriptionDefinition') e.commands.setTextSelection(pos + 1)
    })
    e.view.focus()
  })
  const before = await contents(page)
  await page.keyboard.press('Backspace')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name))
    .toBe('descriptionTerm')
  expect(await contents(page)).toBe(before)
  await expect(page.locator('.widget-descriptionList')).toHaveCount(1)
})

import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('le badge est un objet accessible ; son contexte reste distinct du mot incorrect voisin', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'courrrier' },
            { type: 'footnote', attrs: { text: 'Une précision.' } },
            { type: 'text', text: ' suite' },
          ],
        },
      ],
    })
    e.commands.setTextSelection(4)
    e.view.focus()
  })
  const note = page.getByRole('button', {
    name: 'Note de bas de page 1 : Une précision.',
    exact: true,
  })
  await expect(note).toHaveText('Note 1')
  await expect(note).toHaveAttribute('contenteditable', 'false')
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
  await page.keyboard.press('Control+Enter')
  await expect(page.locator('#proofing-band')).toBeVisible()
  await page.keyboard.press('Escape')
  // At the left boundary the word owns the contextual command, never the badge.
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(10)
    e.view.focus()
  })
  await expect(page.locator('.inline-object-active')).toHaveCount(0)
  await page.keyboard.press('Control+Enter')
  await expect(page.locator('#proofing-band')).toBeVisible()
  await expect(page.locator('#properties')).not.toBeVisible()
  await page.keyboard.press('Escape')
  await page.keyboard.press('ArrowRight')
  await expect(page.locator('.inline-object-active')).toHaveCount(1)
  await expect(page.locator('.inline-object-active .inline-object-label')).toHaveText('Note 1')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
  await expect(page.locator('#proofing-band')).toHaveCount(0)
  await page.keyboard.press('Escape')
  // Selecting the object is distinct from a caret after it; Enter/Space still work.
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setNodeSelection(10)
    e.view.focus()
  })
  await expect(page.locator('.inline-object-active')).toHaveCount(0)
  await page.keyboard.press('Control+Enter')
  await expect(page.locator('#properties')).not.toBeVisible()
  await expect(page.locator('#proofing-band')).toHaveCount(0)
  await note.click()
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
  await page.keyboard.press('Escape')
  expect(
    await page.evaluate(
      () => window.__spike!.editor.state.selection.to - window.__spike!.editor.state.selection.from,
    ),
  ).toBe(1)
  // Keyboard access to the badge uses the same object selection and band.
  await note.focus()
  await page.keyboard.press('Space')
  await expect(page.getByLabel('Texte de la note', { exact: true })).toBeFocused()
})

test('une note modifiée ailleurs ne peut pas être supprimée depuis ses anciens paramètres', async ({
  page,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent('<p>Avant<span data-footnote="Ancien texte"></span> après</p>')
  })
  await page.locator('.footnote-ref .inline-object-label').click()
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'footnote')
        e.view.dispatch(e.state.tr.setNodeMarkup(pos, undefined, { text: 'Modification distante' }))
    })
  })
  await page.locator('#remove-block').click()
  await expect(page.locator('#property-error')).toContainText('modifié ailleurs')
  await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'settings')
  await expect(page.locator('.footnote-ref .inline-object-label')).toHaveCount(1)
  await page.keyboard.press('Escape')
  await page.locator('.footnote-ref .inline-object-label').click()
  await expect(page.getByLabel('Texte de la note', { exact: true })).toHaveValue(
    'Modification distante',
  )
})

test('le badge actif est inversé uniquement à droite et le raccourci suit ce repère entre deux notes', async ({
  page,
}) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme })
    await page.evaluate(() => {
      const e = window.__spike!.editor
      e.commands.setContent(
        '<p>Texte<span data-footnote="A"></span><span data-footnote="B"></span> suite</p>',
      )
      e.commands.setTextSelection(6)
      e.view.focus()
    })
    const original = await page.evaluate(() => window.__spike!.editor.getJSON())
    const label = page.locator('.inline-object-label').first()
    const normal = await label.evaluate((el) => {
      const rect = el.getBoundingClientRect(),
        style = getComputedStyle(el)
      return {
        color: style.color,
        background: style.backgroundColor,
        width: rect.width,
        height: rect.height,
      }
    })
    await expect(page.locator('.inline-object-active')).toHaveCount(0)
    await page.keyboard.press('Control+Enter')
    await expect(page.locator('#properties')).not.toBeVisible()
    await page.keyboard.press('ArrowRight')
    await expect(page.locator('.inline-object-active .inline-object-label')).toHaveText('Note 1')
    const active = await label.evaluate((el) => {
      const rect = el.getBoundingClientRect(),
        style = getComputedStyle(el)
      return {
        color: style.color,
        background: style.backgroundColor,
        width: rect.width,
        height: rect.height,
      }
    })
    expect(active.background).toBe(normal.color)
    expect(active.color).not.toBe(normal.color)
    expect(active.background).not.toBe(normal.background)
    expect(active.width).toBe(normal.width)
    expect(active.height).toBe(normal.height)
    await page.keyboard.press('Meta+Enter')
    await expect(page.getByLabel('Texte de la note', { exact: true })).toHaveValue('A')
    await page.keyboard.press('Escape')
    await page.keyboard.press('ArrowRight')
    await expect(page.locator('.inline-object-active .inline-object-label')).toHaveText('Note 2')
    await expect(label).toHaveCSS('background-color', normal.background)
    await page.keyboard.press('Control+Enter')
    await expect(page.getByLabel('Texte de la note', { exact: true })).toHaveValue('B')
    await page.keyboard.press('Escape')
    await page.keyboard.press('ArrowRight')
    expect(await page.evaluate(() => window.__spike!.editor.state.selection.from)).toBe(9)
    await expect(page.locator('.inline-object-active')).toHaveCount(0)
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(original)
  }
})

for (const context of ['prose', 'quote'] as const) {
  test(`suppression protégée d’une note dans ${context}, annulation, confirmation et undo`, async ({
    page,
  }) => {
    await page.evaluate((kind) => {
      const e = window.__spike!.editor
      const p = {
        type: 'paragraph',
        content: [
          { type: 'text', text: 'Avant' },
          { type: 'footnote', attrs: { text: 'À conserver.' } },
          { type: 'text', text: ' après' },
        ],
      }
      e.commands.setContent({
        type: 'doc',
        content:
          kind === 'quote' ? [{ type: 'quote', content: [p, { type: 'quoteAttribution' }] }] : [p],
      })
      e.view.focus()
    }, context)
    const before = await page.evaluate(() => window.__spike!.editor.getJSON())
    for (const key of ['Backspace', 'Delete', 'Control+Backspace', 'Meta+Backspace']) {
      await page.evaluate((key) => {
        const e = window.__spike!.editor
        e.state.doc.descendants((node, pos) => {
          if (node.type.name === 'footnote')
            e.commands.setTextSelection(pos + (key === 'Delete' ? 0 : 1))
        })
        e.view.focus()
      }, key)
      await page.keyboard.press(key)
      await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'remove')
      await expect(page.locator('#property-title')).toHaveText('Supprimer la note de bas de page')
      expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
      await page.keyboard.press('Escape')
      await expect(page.locator('.tiptap')).toBeFocused()
      expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    }
    // The properties button goes through the very same confirmation.
    await page.locator('.footnote-ref .inline-object-label').click()
    await page.locator('#remove-block').click()
    await expect(page.locator('#properties')).toHaveAttribute('data-intent', 'remove')
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    await page.keyboard.press('Enter')
    await expect(page.locator('.footnote-ref .inline-object-label')).toHaveCount(0)
    await expect(page.locator('.tiptap')).toBeFocused()
    const body = context === 'quote' ? '.tiptap blockquote p' : '.tiptap > p:first-child'
    await expect(page.locator(body)).toContainText('Avant après')
    await page.keyboard.press('Control+z')
    await expect(page.locator('.footnote-ref .inline-object-label')).toHaveCount(1)
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  })
}

test('les numéros suivent les insertions, suppressions et annulations distantes sans interrompre la saisie d’une note', async ({
  page,
  context,
}) => {
  await page.evaluate(async () => {
    const e = window.__spike!.editor
    e.commands.setContent(
      '<p>Texte<span data-footnote="A"></span></p><p>Suite<span data-footnote="B"></span></p>',
    )
    await window.__spike!.provider.flush()
  })
  const peer = await context.newPage()
  await peer.goto('/?client=numbering-peer')
  await peer.waitForFunction(() => window.__spike?.ready)
  await expect(peer.locator('.footnote-ref .inline-object-label')).toHaveText(['Note 1', 'Note 2'])
  await page.locator('.footnote-ref .inline-object-label').nth(1).click()
  const field = page.getByLabel('Texte de la note', { exact: true })
  await expect(field).toBeFocused()
  await peer.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(1)
    e.view.focus()
  })
  await peer.locator('[data-command="insert.footnote"]').click()
  await peer.getByLabel('Texte de la note', { exact: true }).fill('Avant')
  await peer.keyboard.press('Enter')
  for (const view of [page, peer])
    await expect(view.locator('.footnote-ref .inline-object-label')).toHaveText([
      'Note 1',
      'Note 2',
      'Note 3',
    ])
  await expect(field).toBeFocused()
  await expect(field).toHaveValue('B')
  await field.fill('B mise à jour')
  await page.keyboard.press('Enter')
  await expect(peer.locator('.footnote-ref .inline-object-label').nth(2)).toHaveAccessibleName(
    'Note de bas de page 3 : B mise à jour',
  )
  await peer.locator('.footnote-ref .inline-object-label').nth(1).click()
  await peer.locator('#remove-block').click()
  await peer.keyboard.press('Enter')
  for (const view of [page, peer])
    await expect(view.locator('.footnote-ref .inline-object-label')).toHaveText([
      'Note 1',
      'Note 2',
    ])
  await peer.keyboard.press('Control+z')
  for (const view of [page, peer])
    await expect(view.locator('.footnote-ref .inline-object-label')).toHaveText([
      'Note 1',
      'Note 2',
      'Note 3',
    ])
  await expect(page.locator('.footnote-ref .inline-object-label').nth(2)).toHaveAccessibleName(
    'Note de bas de page 3 : B mise à jour',
  )
  await peer.close()
})

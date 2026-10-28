import { expect, test, type Page } from '@playwright/test'

const errors = new WeakMap<Page, string[]>()

test.beforeEach(async ({ page, request }) => {
  const recorded: string[] = []
  errors.set(page, recorded)
  page.on('pageerror', (error) => recorded.push(error.stack || error.message))
  const reset = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await reset.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})
test.afterEach(async ({ page }) => {
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  )
  expect(errors.get(page)).toEqual([])
})
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
function publications(page: Page) {
  const commands: { id: string; params: any }[] = []
  page.on('request', (request) => {
    if (request.url().endsWith('/__spike/invoke')) {
      const input = request.postDataJSON()
      if (input.command === 'commit_structured_commands')
        commands.push(...input.args.request.commands)
    }
  })
  return commands
}
test('remplacer un bandeau annule sa préparation sans modifier l’ancien objet ni fermer le nouveau', async ({
  page,
}) => {
  const image = body(page).locator('[data-editor-widget=figure] img')
  const original = await image.getAttribute('alt')
  const commands = publications(page)
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await page.getByLabel('Texte alternatif', { exact: true }).fill('Ne pas publier')
  let replaced = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'prepare_structured_commands' && !replaced) {
      replaced = true
      const response = await route.fetch()
      await page.getByRole('button', { name: 'Paramètres · Tableau', exact: true }).click()
      await page.evaluate(() => (document.activeElement as HTMLElement)?.blur())
      await route.fulfill({ response })
    } else await route.continue()
  })
  await page.locator('#apply-properties').click()
  await expect(page.locator('#table-properties')).toBeVisible()
  await expect(page.locator('#apply-properties')).toBeEnabled()
  await expect.poll(() => page.evaluate(() => window.__spike!.provider.historyBusy)).toBe(false)
  await expect(image).toHaveAttribute('alt', original!)
  expect(commands).toEqual([])
})
test('image : remplacement et paramètres préservent la légende, les identités et les ressources pour annuler', async ({
  page,
}) => {
  await page.evaluate(() => {
    const root = window.__spike!.provider.doc.getXmlFragment('default')
    const image = root
      .toArray()
      .find(
        (node) => (node as import('yjs').XmlElement).nodeName === 'figure',
      ) as import('yjs').XmlElement
    ;(window as any).__savedImage = {
      node: image,
      caption: image.get(0),
      src: image.getAttribute('src'),
    }
  })
  const commands = publications(page)
  const caption = await body(page).locator('[data-editor-widget=figure] figcaption').innerText()
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await page.getByLabel('Texte alternatif', { exact: true }).fill('Image remplacée')
  await page.locator('#image-file').setInputFiles({
    name: 'replacement.svg',
    mimeType: 'image/svg+xml',
    buffer: Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" width="50" height="30"><rect width="50" height="30" fill="blue"/></svg>',
    ),
  })
  // The imported file is a draft field until the whole band is confirmed.
  await expect(page.locator('#replace-image')).toHaveText(
    'Image remplacée — appliquer pour confirmer',
  )
  await page.locator('#apply-properties').click()
  await expect(body(page).locator('[data-editor-widget=figure] img')).toHaveAttribute(
    'alt',
    'Image remplacée',
  )
  expect(commands.at(-1)?.id).toBe('node.set_attrs')
  expect(
    await page.evaluate(() => {
      const image = (window as any).__savedImage
      return (
        window.__spike!.provider.doc.getXmlFragment('default').toArray().includes(image.node) &&
        image.node.get(0) === image.caption &&
        image.node.getAttribute('src') !== image.src
      )
    }),
  ).toBe(true)
  await expect(body(page).locator('[data-editor-widget=figure] figcaption')).toHaveText(caption)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect
    .poll(() =>
      page.evaluate(() => {
        const image = (window as any).__savedImage
        return image.node.getAttribute('src') === image.src
      }),
    )
    .toBe(true)
})
test('widget inséré et supprimé par les cibles communes, une annulation par action', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent('<p>A😀 suite</p>')
    editor.commands.setTextSelection(4)
    editor.view.focus()
    await provider.clearHistory()
  })
  const commands = publications(page)
  await page.getByRole('button', { name: 'Citation en bloc', exact: true }).click()
  const widget = page.locator('[data-editor-widget=quote]')
  await expect(widget).toHaveCount(1)
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  expect(commands.map((command) => command.id)).toEqual(['block.splice'])
  expect(commands[0].params.index).toBe(0)
  await page.keyboard.type('Citation saisie')
  await expect(widget.locator('blockquote')).toContainText('Citation saisie')
  await body(page).press('ControlOrMeta+Backspace')
  await expect(widget).toHaveCount(0)
  expect(commands.at(-1)?.id).toBe('block.remove')
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(widget).toContainText('Citation saisie')
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
})
test('note : insertion UTF-16, paramètres en place, suppression protégée et annulation native', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent('<p>Test😀 des notes</p>')
    editor.commands.setTextSelection(7)
    editor.view.focus()
    await provider.clearHistory()
  })
  const commands = publications(page)
  await page.getByRole('button', { name: 'Insérer une note de bas de page', exact: true }).click()
  const field = page.getByRole('textbox', { name: 'Texte de la note', exact: true })
  await expect(field).toBeFocused()
  await field.fill('Une note')
  await field.press('Enter')
  await expect(page.locator('#properties')).toBeHidden()
  expect(commands[0]).toMatchObject({
    id: 'inline.replace',
    params: { target: { zone: { kind: 'body', path: [0] }, from: 6, to: 6 } },
  })
  expect(commands[1].id).toBe('node.set_attrs')
  await page.evaluate(() => {
    const root = window.__spike!.provider.doc.getXmlFragment('default')
    ;(window as any).__savedNote = (root.get(0) as import('yjs').XmlElement)
      .toArray()
      .find((node) => (node as import('yjs').XmlElement).nodeName === 'footnote')
  })
  await body(page).locator('.footnote-ref .inline-object-label').click()
  await field.fill('Une note revue')
  await field.press('Enter')
  await expect(page.locator('#properties')).toBeHidden()
  expect(
    await page.evaluate(() => {
      const root = window.__spike!.provider.doc.getXmlFragment('default')
      return (root.get(0) as import('yjs').XmlElement)
        .toArray()
        .includes((window as any).__savedNote)
    }),
  ).toBe(true)
  await body(page).press('ControlOrMeta+Enter')
  await expect(field).toBeFocused()
  await expect(field).toHaveValue('Une note revue')
  await field.press('Escape')
  await body(page).press('Backspace')
  await expect(page.locator('#property-title')).toHaveText('Supprimer la note de bas de page')
  await page.locator('#apply-properties').click()
  await expect(body(page).locator('.footnote-ref')).toHaveCount(0)
  await expect(body(page)).toContainText('Test😀 des notes')
  await expect(page.locator('#properties')).toBeHidden()
  expect(commands.at(-1)?.id).toBe('text.delete')
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(body(page).locator('.footnote-ref')).toHaveCount(1)
  await expect(body(page).locator('.footnote-ref .inline-object-label')).toHaveAttribute(
    'title',
    /Une note revue/,
  )
})
test('une proposition de widget périmée est refusée sans publier ni perdre la saisie', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent('<p>Avant après</p>')
    editor.commands.setTextSelection(6)
    editor.view.focus()
    await provider.flush()
  })
  const commands = publications(page)
  let changed = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'prepare_structured_commands' && !changed) {
      changed = true
      const response = await route.fetch()
      await page.evaluate(() => window.__spike!.editor.commands.insertContent(' conservé'))
      await route.fulfill({ response })
    } else await route.continue()
  })
  await page.getByRole('button', { name: 'Citation en bloc', exact: true }).click()
  await expect(page.locator('#message')).toContainText('Le document a changé')
  await expect(page.locator('[data-editor-widget=quote]')).toHaveCount(0)
  await expect(body(page)).toContainText('Avant conservé après')
  expect(commands).toEqual([])
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
})

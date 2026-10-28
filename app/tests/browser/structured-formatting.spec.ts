import { expect, test, type Page } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  const reset = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await reset.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})
async function select(page: Page, from: number, to = from) {
  await page.evaluate(
    ({ from, to }) => {
      const { editor } = window.__spike!
      editor.commands.setTextSelection({ from, to })
      editor.view.focus()
    },
    { from, to },
  )
}
async function cleanHistory(page: Page) {
  await page.evaluate(() => window.__spike!.provider.clearHistory())
}
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
test('styles multizones : lot typé, badges préservés, une annulation et aucune remise en relais', async ({
  page,
}) => {
  await cleanHistory(page)
  const before = await page.evaluate(() => {
    const { editor } = window.__spike!
    editor.commands.selectAll()
    editor.view.focus()
    return editor.getJSON()
  })
  const requests: {
    command: string
    args: { request?: { commands: { id: string; params: any }[] } }
  }[] = []
  page.on('request', (request) => {
    if (request.url().endsWith('/__spike/invoke')) requests.push(request.postDataJSON())
  })
  await page.getByRole('button', { name: 'Gras', exact: true }).click()
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  const batch = requests.find((r) => r.command === 'commit_structured_commands')!.args.request!
    .commands
  expect(batch.length).toBeGreaterThan(5)
  expect(batch.every((c) => c.id === 'text.bold' && c.params.target.zone.kind === 'body')).toBe(
    true,
  )
  expect(batch.some((c) => c.params.target.zone.path.length >= 5)).toBe(true) // a table cell
  expect(requests.filter((r) => r.command === 'relay')).toEqual([])
  await expect(body(page).locator('[data-editor-widget=figure] figcaption strong')).toHaveCount(1)
  const after = await page.evaluate(() => ({
    selection: window.__spike!.editor.state.selection.toJSON(),
    note: window
      .__spike!.editor.getJSON()
      .content!.flatMap((n) => n.content || [])
      .find((n) => n.type === 'footnote'),
  }))
  expect(after.selection.type).toBe('all')
  expect(after.note?.marks).toContainEqual({ type: 'bold' })
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await expect(page.locator('#connection')).toHaveText('Connecté')
})
test('raccourcis, titres, styles de saisie privés et retrait de lien au curseur', async ({
  page,
}) => {
  await page.evaluate(async () => {
    window.__spike!.editor.commands.setContent('<p>A😀é texte</p>')
    await window.__spike!.provider.flush()
  })
  await cleanHistory(page)
  await select(page, 1, 5)
  await body(page).press('ControlOrMeta+b')
  await expect(body(page).locator('strong')).toHaveText('A😀é')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.toJSON()))
    .toMatchObject({ anchor: 1, head: 5 })
  await body(page).press('ControlOrMeta+Alt+2')
  await expect(body(page).locator('h2')).toHaveText('A😀é texte')
  await expect(body(page).locator('h2 strong')).toHaveText('A😀é')
  await body(page).press('ControlOrMeta+Alt+2')
  await expect(body(page).locator('h2')).toHaveCount(0)
  await expect(body(page).locator('p strong')).toHaveText('A😀é')
  await select(page, 5)
  const revision = await page.evaluate(() => window.__spike!.provider.revision)
  await body(page).press('ControlOrMeta+i')
  expect(await page.evaluate(() => window.__spike!.provider.revision)).toBe(revision)
  await page.keyboard.type(' suite')
  await expect(body(page).locator('em')).toHaveText(' suite')
  await select(page, 1, 5)
  await body(page).press('ControlOrMeta+k')
  await page.locator('#parameter-url').fill('https://example.org')
  await page.locator('#parameter-url').press('Enter')
  await expect(body(page).locator('a')).toHaveText('A😀é')
  await select(page, 4)
  await page.getByRole('button', { name: 'Édition', exact: true }).click()
  await page.locator('[data-command="text.unlink"]').click()
  await expect(body(page).locator('a')).toHaveCount(0)
  await expect(body(page).locator('strong')).not.toHaveCount(0)
})
test('une préparation périmée conserve la saisie, la sélection et la connexion', async ({
  page,
}) => {
  await page.evaluate(async () => {
    window.__spike!.editor.commands.setContent('<p>Texte initial</p>')
    await window.__spike!.provider.flush()
  })
  await cleanHistory(page)
  await select(page, 1, 6)
  let intervened = false
  let committed = false
  await page.route('**/__spike/invoke', async (route) => {
    const command = route.request().postDataJSON().command
    if (command === 'prepare_structured_commands' && !intervened) {
      intervened = true
      const response = await route.fetch()
      await page.evaluate(() => {
        const e = window.__spike!.editor
        e.commands.setTextSelection(6)
        e.commands.insertContent(' conservé')
      })
      await route.fulfill({ response })
    } else {
      if (command === 'commit_structured_commands') committed = true
      await route.continue()
    }
  })
  await page.getByRole('button', { name: 'Italique', exact: true }).click()
  await expect(page.locator('#message')).toContainText('Le document a changé')
  expect(committed).toBe(false)
  await expect(body(page)).toContainText('Texte conservé initial')
  await expect(body(page).locator('em')).toHaveCount(0)
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
  await expect(body(page)).toBeEditable()
})

test('effacer la mise en forme ne touche que la sélection et s’annule en un lot', async ({
  page,
}) => {
  await page.evaluate(async () => {
    window.__spike!.editor.commands.setContent('<p><strong>Texte <em>mixte</em></strong> suite</p>')
    await window.__spike!.provider.flush()
  })
  await cleanHistory(page)
  await select(page, 1, 6)
  await page.getByRole('button', { name: 'Édition', exact: true }).click()
  await page.locator('[data-command="text.clear"]').click()
  await expect(body(page).locator('strong')).toHaveText(' mixte')
  await expect(body(page).locator('em')).toHaveText('mixte')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(body(page).locator('strong')).toHaveText('Texte mixte')
})

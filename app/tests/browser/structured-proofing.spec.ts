import { expect, test, type Page } from '@playwright/test'

const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
test.beforeEach(async ({ page, request }) => {
  const reset = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await reset.json()).error).toBeUndefined()
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})
function publications(page: Page) {
  const requests: {
    command: string
    args: { request?: { commands: { id: string; params: any }[] } }
  }[] = []
  page.on('request', (request) => {
    if (request.url().endsWith('/__spike/invoke')) requests.push(request.postDataJSON())
  })
  return requests
}
async function search(page: Page, word: string, text: string) {
  await page.getByRole('button', { name: 'Rechercher', exact: true }).click()
  await page.getByRole('combobox', { name: 'Rechercher', exact: true }).fill(word)
  await page.getByRole('textbox', { name: 'Remplacer par', exact: true }).fill(text)
}
async function spelling(page: Page) {
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'spelling_suggestions')
      await route.fulfill({ json: { result: ['courrier', 'courriers'] } })
    else await route.continue()
  })
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'quote',
          content: [
            {
              type: 'paragraph',
              content: [
                { type: 'text', text: 'Ce courrrier est prêt.', marks: [{ type: 'bold' }] },
              ],
            },
            { type: 'quoteAttribution' },
          ],
        },
      ],
    })
    editor.commands.setTextSelection(7)
    editor.view.focus()
    await provider.flush()
    await provider.clearHistory()
  })
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
  await body(page).press('ControlOrMeta+Enter')
  await expect(page.locator('#proofing-band [data-default]')).toHaveText('courrier')
}

test('tout remplacer publie un lot multizone unique, conserve les styles et badges et s’annule en une étape', async ({
  page,
  context,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: '😀 mot ', marks: [{ type: 'bold' }] },
            { type: 'text', text: 'MOT', marks: [{ type: 'italic' }] },
            { type: 'footnote', attrs: { text: 'mot privé' } },
          ],
        },
        {
          type: 'quote',
          content: [
            { type: 'paragraph', content: [{ type: 'text', text: 'mot' }] },
            { type: 'quoteAttribution', content: [{ type: 'text', text: 'mot' }] },
          ],
        },
        {
          type: 'tableFigure',
          content: [
            {
              type: 'table',
              content: [
                {
                  type: 'tableRow',
                  content: [
                    {
                      type: 'tableCell',
                      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'mot' }] }],
                    },
                  ],
                },
              ],
            },
            { type: 'tableCaption', content: [{ type: 'text', text: 'mot' }] },
          ],
        },
      ],
    })
    await provider.flush()
    await provider.clearHistory()
  })
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  const peer = await context.newPage()
  await peer.goto('/?client=proofing-peer')
  await peer.waitForFunction(() => window.__spike?.ready)
  const requests = publications(page)
  await search(page, 'mot', '<suite>')
  await expect(page.locator('.search-hit')).toHaveCount(6)
  await page.getByRole('button', { name: 'Tout remplacer', exact: true }).click()
  await expect(page.locator('.search-hit')).toHaveCount(0)
  await expect(body(page).locator('strong')).toHaveText('😀 <suite> ')
  await expect(body(page).locator('em')).toHaveText('<suite>')
  await expect(body(page).locator('.inline-object[data-kind=footnote]')).toHaveCount(1)
  const after = await page.evaluate(() => window.__spike!.editor.getJSON())
  await expect.poll(() => peer.evaluate(() => window.__spike!.editor.getJSON())).toEqual(after)
  const batch = requests.filter((r) => r.command === 'commit_structured_commands')
  expect(batch).toHaveLength(1)
  expect(batch[0].args.request!.commands).toHaveLength(6)
  expect(batch[0].args.request!.commands.every((c) => c.id === 'text.insert')).toBe(true)
  expect(requests.filter((r) => r.command === 'relay')).toEqual([])
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await expect.poll(() => peer.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await peer.close()
})

test('un remplacement périmé conserve la saisie et les champs de recherche sans publier de lot partiel', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent('<p>mot mot</p>')
    await provider.flush()
    await provider.clearHistory()
  })
  await search(page, 'mot', 'remplacement')
  let interrupted = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'prepare_structured_commands' && !interrupted) {
      interrupted = true
      const response = await route.fetch()
      await page.evaluate(() => window.__spike!.editor.commands.insertContentAt(1, 'conservé '))
      await route.fulfill({ response })
    } else await route.continue()
  })
  const requests = publications(page)
  await page.getByRole('button', { name: 'Tout remplacer', exact: true }).click()
  await expect(page.locator('#side-content [role=alert]')).toContainText('Le document a changé')
  await expect(body(page)).toContainText('conservé mot mot')
  await expect(page.getByRole('textbox', { name: 'Remplacer par', exact: true })).toHaveValue(
    'remplacement',
  )
  expect(requests.filter((r) => r.command === 'commit_structured_commands')).toEqual([])
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
  await expect(page.locator('#connection')).toHaveText('Connecté')
  // The same search stays usable after the refusal; empty replacements are deletions.
  await page.getByRole('textbox', { name: 'Remplacer par', exact: true }).fill('')
  await page.getByRole('button', { name: 'Tout remplacer', exact: true }).click()
  await expect(page.locator('.search-hit')).toHaveCount(0)
  expect(
    requests
      .find((r) => r.command === 'commit_structured_commands')!
      .args.request!.commands.every((c) => c.id === 'text.delete'),
  ).toBe(true)
})

test('effacer la première occurrence répare le paragraphe vide et conserve une position de saisie valide', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent('<p>mot</p><p>suite</p>')
    editor.commands.setTextSelection({ from: 1, to: 4 })
    await provider.flush()
    await provider.clearHistory()
  })
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  await search(page, 'mot', '')
  const requests = publications(page)
  await page.getByRole('button', { name: 'Remplacer', exact: true }).click()
  await expect(page.locator('.search-hit')).toHaveCount(0)
  expect(
    await page.evaluate(() => ({
      first: window.__spike!.editor.state.doc.firstChild?.textContent,
      caret: window.__spike!.editor.state.selection.from,
      end: window.__spike!.editor.state.doc.lastChild?.content.size,
    })),
  ).toEqual({ first: 'suite', caret: 1, end: 0 })
  await expect(page.locator('#side-content [role=alert]')).toBeEmpty()
  expect(requests.filter((r) => r.command === 'relay')).toEqual([])
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
})

test('la correction dans un widget utilise text.insert, garde le gras et restitue le curseur au clavier', async ({
  page,
}) => {
  await spelling(page)
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  const requests = publications(page)
  await page.locator('#proofing-band').press('Enter')
  await expect(page.locator('#proofing-band')).toHaveCount(0)
  await expect(body(page).locator('blockquote strong')).toHaveText('Ce courrier est prêt.')
  await expect(body(page)).toBeFocused()
  expect(
    requests.find((r) => r.command === 'commit_structured_commands')!.args.request!.commands,
  ).toMatchObject([
    {
      id: 'text.insert',
      params: { target: { zone: { kind: 'body', path: [0, 0] } }, marks: [{ type: 'bold' }] },
    },
  ])
  expect(requests.filter((r) => r.command === 'relay')).toEqual([])
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
})

test('ignorer un mot modifie seulement les exceptions documentaires et s’annule', async ({
  page,
}) => {
  await spelling(page)
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  const requests = publications(page)
  await page.getByRole('button', { name: 'Ignorer dans ce document', exact: true }).click()
  await expect(page.locator('#proofing-band')).toHaveCount(0)
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toHaveCount(0)
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  expect(
    requests.find((r) => r.command === 'commit_structured_commands')!.args.request!.commands[0].id,
  ).toBe('spelling.ignore')
  expect(requests.filter((r) => r.command === 'relay')).toEqual([])
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.historyState.undo_steps))
    .toBe(1)
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(page.locator('.spelling-error[data-word="courrrier"]')).toBeVisible({
    timeout: 15000,
  })
})

test('fermer une correction pendant sa préparation conserve le mot et ne ferme pas le nouveau bandeau', async ({
  page,
}) => {
  await spelling(page)
  let interrupted = false
  let completed = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'prepare_structured_commands' && !interrupted) {
      interrupted = true
      const response = await route.fetch()
      await page.locator('#proofing-band').press('Escape')
      await body(page).press('ControlOrMeta+Enter')
      await route.fulfill({ response })
      completed = true
    } else await route.continue()
  })
  const requests = publications(page)
  await page.locator('#proofing-band').press('Enter')
  await expect.poll(() => completed).toBe(true)
  await expect(page.locator('#proofing-band')).toBeVisible()
  await expect.poll(() => interrupted).toBe(true)
  await expect(page.locator('#proofing-band [data-default]')).toHaveText('courrier')
  await expect(body(page)).toContainText('courrrier')
  expect(requests.filter((r) => r.command === 'commit_structured_commands')).toEqual([])
})

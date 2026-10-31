import { expect, test, type Page } from '@playwright/test'
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
async function ready(page: Page, id: string) {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
test.beforeEach(async ({ request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
})
test(
  'active en un clic sans remplacer le document, sa sélection ni son historique',
  { tag: '@smoke' },
  async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await ready(page, 'main')
    await body(page).fill('Rédaction commencée en individuel')
    await page.evaluate(async () => {
      await window.__spike!.provider.clearHistory()
    })
    await body(page).press('End')
    await page.keyboard.type(' puis poursuivie')
    await body(page).press('ControlOrMeta+i')
    const before = await page.evaluate(async () => {
      const { provider, editor } = window.__spike!
      await provider.flush()
      return {
        clientID: provider.doc.clientID,
        generation: provider.generation,
        revision: provider.revision,
        steps: provider.historyState.undo_steps,
        selection: editor.state.selection.toJSON(),
        styles: editor.state.storedMarks?.map((mark) => mark.toJSON()),
        resources: [...provider.resources],
      }
    })
    expect(before.steps).toBeGreaterThan(0)
    expect(await page.evaluate(() => !!window.__spike!.provider.awareness)).toBe(false)
    await page.locator('#synchronization').click()
    await expect(page.locator('#synchronization')).toHaveAttribute('aria-pressed', 'true')
    await expect(body(page)).toBeFocused()
    const after = await page.evaluate(() => {
      const { provider, editor } = window.__spike!
      return {
        clientID: provider.doc.clientID,
        generation: provider.generation,
        revision: provider.revision,
        steps: provider.historyState.undo_steps,
        selection: editor.state.selection.toJSON(),
        styles: editor.state.storedMarks?.map((mark) => mark.toJSON()),
        resources: [...provider.resources],
      }
    })
    expect(after).toEqual(before)
    await body(page).press('ControlOrMeta+z')
    await expect(body(page)).not.toContainText('puis poursuivie')
    await body(page).press('ControlOrMeta+Shift+z')
    await expect(body(page)).toContainText('puis poursuivie')
    await page.locator('#synchronization').click()
    await expect(page.locator('#synchronization')).toHaveAttribute('aria-pressed', 'false')
    expect(await page.evaluate(() => !!window.__spike!.provider.awareness)).toBe(false)
    expect(errors).toEqual([])
  },
)
test('la seconde fenêtre active les services et son départ permet de revenir en individuel', async ({
  page,
  context,
}) => {
  await ready(page, 'main')
  await body(page).fill('Texte partagé')
  await page.evaluate(async () => {
    await window.__spike!.provider.flush()
  })
  const peer = await context.newPage()
  await ready(peer, 'peer-sync')
  for (const view of [page, peer]) {
    await expect(view.locator('#synchronization')).toHaveAttribute('aria-pressed', 'true')
    await expect(view.locator('#synchronization')).toBeDisabled()
    await expect
      .poll(() => view.evaluate(() => window.__spike!.provider.awareness?.getStates().size))
      .toBe(2)
  }
  await body(peer).press('Home')
  await expect
    .poll(() => peer.evaluate(() => !!window.__spike!.provider.awareness?.getLocalState()?.cursor))
    .toBe(true)
  await expect(page.locator('.collaboration-carets__caret').first()).toBeVisible()
  await body(peer).press('End')
  await peer.keyboard.type(' en commun')
  await expect(body(page)).toContainText('en commun')
  await peer.evaluate(async () => {
    await window.__spike!.provider.disconnect()
  })
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.awareness?.getStates().size))
    .toBe(1)
  await peer.evaluate(async () => {
    await window.__spike!.provider.reconnect()
  })
  await body(peer).press('Home')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.awareness?.getStates().size))
    .toBe(2)
  await expect(page.locator('.collaboration-carets__caret').first()).toBeVisible()
  await peer.close()
  await expect(page.locator('#synchronization')).toBeEnabled()
  expect(await page.evaluate(() => window.__spike!.provider.synchronization?.enabled)).toBe(true)
  await expect
    .poll(() => page.evaluate(() => window.__spike!.provider.awareness?.getStates().size))
    .toBe(1)
  await page.locator('#synchronization').click()
  await expect(page.locator('#synchronization')).toHaveAttribute('aria-pressed', 'false')
})

test('le clic pendant la préédition est différé sans déplacer le focus ni partager du texte incomplet', async ({
  page,
}) => {
  await ready(page, 'main')
  await body(page).fill('Base')
  await page.evaluate(async () => {
    await window.__spike!.provider.clearHistory()
  })
  const revision = await page.evaluate(() => {
    const { provider, editor } = window.__spike!
    editor.view.dom.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }))
    editor.commands.insertContentAt(3, '日本')
    return provider.revision
  })
  await page.locator('#synchronization').click()
  await expect(page.locator('#synchronization')).toContainText('Activation après la saisie')
  expect(await page.evaluate(() => window.__spike!.provider.revision)).toBe(revision)
  expect(await page.evaluate(() => !!window.__spike!.provider.awareness)).toBe(false)
  await expect(body(page)).toBeFocused()
  await page.evaluate(() =>
    window.__spike!.editor.view.dom.dispatchEvent(
      new CompositionEvent('compositionend', { bubbles: true, data: '日本' }),
    ),
  )
  await expect(page.locator('#synchronization')).toHaveAttribute('aria-pressed', 'true')
  await expect(body(page)).toContainText('Ba日本se')
  await expect(body(page)).toBeFocused()
  await body(page).press('ControlOrMeta+z')
  await expect(body(page)).toContainText('Base')
  await expect(body(page)).not.toContainText('日本')
})

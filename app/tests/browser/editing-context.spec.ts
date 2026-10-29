import { expect, test, type Page } from '@playwright/test'
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
async function ready(page: Page, id = 'main') {
  await page.goto(`/?client=${id}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
async function snapshot(page: Page) {
  return page.evaluate(async () => {
    const p = window.__spike!.provider
    await p.flush(false)
    return p.transport.invoke<any>('editing_context', { generation: p.generation })
  })
}
test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await ready(page)
})
test('navigation et styles de frappe sont privés, sans révision, historique ni rendu', async ({
  page,
}) => {
  await body(page).fill('A😀é日本')
  await page.evaluate(async () => window.__spike!.provider.clearHistory())
  const before = await snapshot(page)
  let relays = 0
  const requests: any[] = []
  page.on('request', (request) => {
    if (!request.url().endsWith('/__spike/invoke')) return
    const input = request.postDataJSON()
    if (input.command === 'relay') relays++
    if (input.command === 'set_editing_context') requests.push(input.args.request)
  })
  await page.evaluate(() => window.__spike!.editor.commands.setTextSelection({ from: 7, to: 2 }))
  const selected = await snapshot(page)
  expect(selected.selection).toEqual({
    kind: 'text',
    anchor: { zone: { kind: 'body', path: [0] }, offset: 6 },
    head: { zone: { kind: 'body', path: [0] }, offset: 1 },
  })
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(7)
    e.view.focus()
  })
  await body(page).press('ControlOrMeta+b')
  await body(page).press('ControlOrMeta+i')
  const styled = await snapshot(page)
  expect(styled.typing_styles).toEqual([{ type: 'bold' }, { type: 'italic' }])
  expect(styled.guard).toEqual(before.guard)
  expect(styled.undo_steps).toBe(0)
  expect(styled.redo_steps).toBe(0)
  expect(relays).toBe(0)
  expect(requests.length).toBeGreaterThan(0)
  expect(requests.every((r) => !('update' in r) && Array.isArray(r.frontier))).toBe(true)
  await page.keyboard.type('Texte')
  await snapshot(page)
  await expect(body(page).locator('strong em, em strong')).toContainText('Texte')
  await body(page).press('ControlOrMeta+b')
  await body(page).press('ControlOrMeta+i')
  expect((await snapshot(page)).typing_styles).toEqual([])
  await page.keyboard.type(' simple')
  await snapshot(page)
  await expect(body(page).locator('strong')).not.toContainText('simple')
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
})
test('deux fenêtres conservent leurs styles et leur sélection sans déplacement de focus distant', async ({
  page,
  context,
}) => {
  await body(page).fill('Base')
  await snapshot(page)
  const peer = await context.newPage()
  await ready(peer, 'context-peer')
  await body(page).press('End')
  await body(page).press('ControlOrMeta+b')
  await snapshot(page)
  await peer.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(5)
    e.view.focus()
  })
  await body(peer).press('ControlOrMeta+i')
  await snapshot(peer)
  expect((await snapshot(page)).typing_styles).toEqual([{ type: 'bold' }])
  expect((await snapshot(peer)).typing_styles).toEqual([{ type: 'italic' }])
  await page.keyboard.type(' Premier')
  await snapshot(page)
  await expect(body(peer)).toContainText('Premier')
  await expect(body(peer)).toBeFocused()
  expect((await snapshot(peer)).typing_styles).toEqual([{ type: 'italic' }])
  await peer.keyboard.type(' Second')
  await snapshot(peer)
  await expect(body(page)).toContainText('Second')
  await expect(body(page)).toBeFocused()
  await expect(body(peer).locator('em')).toContainText('Second')
  await expect(body(page).locator('strong')).toContainText('Premier')
  await peer.close()
})
test('les zones riches d’en-tête partagent le contexte, les brouillons détachés restent privés', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  await page.locator('dialog select').first().selectOption('article')
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page.getByRole('button', { name: 'Créer', exact: true }).click(),
  ])
  await page.waitForFunction(() => window.__spike?.ready)
  await page.locator('#metadata-tab-titles').click()
  const title = page.getByRole('textbox', { name: 'Titre', exact: true })
  await title.fill('Titre japonais 日本')
  await title.press('End')
  await title.press('ControlOrMeta+i')
  const s = await snapshot(page)
  expect(s.selection.kind).toBe('text')
  expect(s.selection.head.zone).toMatchObject({
    kind: 'record',
    collection: 'localizations',
    field: 'title',
    path: [0],
  })
  expect(s.typing_styles).toEqual([{ type: 'italic' }])
  await page.locator('#metadata-tab-authors').click()
  await page.getByRole('button', { name: 'Ajouter · Auteur', exact: true }).last().click()
  const draft = page.locator('.metadata-record-draft')
  await expect(draft).toBeVisible()
  await draft.getByRole('textbox').first().fill('Nom privé')
  expect((await snapshot(page)).selection).toEqual(s.selection)
  await page.keyboard.press('Escape')
  await expect(draft).toHaveCount(0)
})
test('une publication concurrente invalide une navigation en retard sans suspendre la saisie', async ({
  page,
  context,
}) => {
  await body(page).fill('Base')
  await snapshot(page)
  const peer = await context.newPage()
  await ready(peer, 'context-conflict')
  let raced = false
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'set_editing_context' && !raced) {
      raced = true
      await body(peer).press('Home')
      await peer.keyboard.type('Avant ')
      await snapshot(peer)
    }
    await route.continue()
  })
  await body(page).press('End')
  await body(page).press('ArrowLeft')
  await snapshot(page)
  expect(raced).toBe(true)
  await expect(body(page)).toContainText('Avant Base')
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
  await page.keyboard.type('X')
  await snapshot(page)
  await expect(body(peer)).toContainText('X')
  await peer.close()
})

test('les sélections de badges et de cellules gardent leurs cibles structurelles', async ({
  page,
}) => {
  const paths = await page.evaluate(() => {
    const e = window.__spike!.editor
    let badge = -1
    const cells: number[] = []
    const path = (pos: number) => {
      const r = e.state.doc.resolve(pos)
      return Array.from({ length: r.depth + 1 }, (_, depth) => r.index(depth))
    }
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'footnote') badge = pos
      if (['tableCell', 'tableHeader'].includes(node.type.name)) cells.push(pos)
    })
    e.view.focus()
    e.commands.setNodeSelection(badge)
    return {
      badge: path(badge),
      cells: [path(cells[0]), path(cells[1])],
      positions: cells.slice(0, 2),
    }
  })
  expect((await snapshot(page)).selection).toEqual({
    kind: 'node',
    target: { root: { kind: 'body' }, path: paths.badge },
  })
  await page.evaluate((cells) => {
    window.__spike!.editor.commands.setCellSelection({ anchorCell: cells[1], headCell: cells[0] })
  }, paths.positions)
  expect((await snapshot(page)).selection).toEqual({
    kind: 'cells',
    anchor: { root: { kind: 'body' }, path: paths.cells[1] },
    head: { root: { kind: 'body' }, path: paths.cells[0] },
  })
})

test('une zone supprimée à distance ne devient pas automatiquement son voisin dans le contexte', async ({
  page,
  context,
}) => {
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setContent({
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Premier' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'Second' }] },
        { type: 'paragraph' },
      ],
    })
    e.view.focus()
    e.commands.setTextSelection(3)
  })
  expect((await snapshot(page)).selection.head.offset).toBe(2)
  const peer = await context.newPage()
  await ready(peer, 'context-removal')
  await peer.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.deleteRange({ from: 0, to: e.state.doc.firstChild!.nodeSize })
  })
  await snapshot(peer)
  await expect(body(page)).not.toContainText('Premier')
  const lost = await snapshot(page)
  expect(lost.selection_lost).toBe(true)
  expect(lost.selection).toBeNull()
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(2)
    e.view.focus()
  })
  expect((await snapshot(page)).selection_lost).toBe(false)
  await peer.close()
})

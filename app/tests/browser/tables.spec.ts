import type { JSONContent } from '@tiptap/core'
import { expect, test, type Page } from '@playwright/test'

async function blank(page: Page) {
  await page.request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(() => {
    const editor = window.__spike!.editor
    editor.commands.setContent({
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Avant après' }] }],
    })
    editor.commands.setTextSelection(6)
  })
}
async function create(page: Page, rows = '2', columns = '2') {
  await page.getByRole('button', { name: 'Insérer un tableau', exact: true }).click()
  const panel = page.locator('#table-properties')
  await expect(panel).toBeVisible()
  await panel.getByRole('spinbutton', { name: 'Lignes', exact: true }).fill(rows)
  await panel.getByRole('spinbutton', { name: 'Colonnes', exact: true }).fill(columns)
  await panel.getByRole('spinbutton', { name: 'Colonnes', exact: true }).press('Enter')
  await expect(page.locator('[data-editor-widget="tableFigure"]')).toHaveCount(1)
  return page.locator('[data-editor-widget="tableFigure"]')
}
test('un seul widget, légende riche, navigation clavier et insertion au curseur', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page)
  await expect(widget.locator('[data-editor-widget]')).toHaveCount(0)
  await expect(widget.locator('tr')).toHaveCount(2)
  await expect(widget.locator('th')).toHaveCount(2)
  await page.keyboard.type('Contenu')
  await expect(widget.locator('th').first()).toContainText('Contenu')
  await page.keyboard.press('Enter')
  await page.keyboard.type('suite')
  await expect(widget.locator('th').first().locator('p')).toHaveCount(1)
  await widget.locator('td').last().click()
  await expect
    .poll(() =>
      page.evaluate(() => {
        const pos = window.__spike!.editor.state.selection.$from
        return [pos.index(2), pos.index(3)]
      }),
    )
    .toEqual([1, 1])
  await page.keyboard.press('Tab')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name))
    .toBe('tableCaption')
  await page.keyboard.type('Ma légende')
  await expect(widget.locator('figcaption')).toHaveText('Ma légende')
  await page.keyboard.press('Shift+Tab')
  await page.keyboard.type('Dernière cellule')
  await expect(widget.locator('td').last()).toContainText('Dernière cellule')
  await page.keyboard.press('Tab')
  await page.keyboard.press('Tab')
  await page.keyboard.type('Suite ')
  const doc = (await page.evaluate(() => window.__spike!.editor.getJSON())) as JSONContent
  expect(doc.content?.map((n) => n.type)).toEqual([
    'paragraph',
    'tableFigure',
    'paragraph',
    'paragraph',
    'paragraph',
  ])
  expect(doc.content?.[0].content?.[0].text).toBe('Avant')
  expect(doc.content?.[3].content?.[0].text).toContain('après')
  // The active editor stays a single column; no duplicate tool strip returns.
  await expect(page.locator('.table-tools')).toHaveCount(0)
})
test('bandeau commun : opérations autour de la cellule active, annulation, suppression et limites', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page)
  await widget.locator('td').first().click()
  await page.keyboard.type('À conserver')
  const settings = widget.getByRole('button', { name: 'Paramètres · Tableau', exact: true })
  await settings.click()
  const panel = page.locator('#table-properties')
  await expect(panel).toBeVisible()
  const frame = await widget.boundingBox(),
    band = await panel.boundingBox()
  expect(Math.abs(frame!.width - band!.width)).toBeLessThan(4)
  expect(band!.y).toBeGreaterThan(frame!.y)
  await expect(panel.getByRole('spinbutton')).toHaveCount(0)
  await panel.getByRole('radio', { name: 'Ajouter une colonne à gauche', exact: true }).click()
  await panel.getByRole('button', { name: 'Valider', exact: true }).press('Enter')
  await expect(widget.locator('td')).toHaveCount(3)
  await expect(widget.locator('td').nth(1)).toContainText('À conserver')
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z')
  await expect(widget.locator('td')).toHaveCount(2)
  await expect(widget.locator('td').first()).toContainText('À conserver')
  await settings.click()
  await panel.getByRole('radio', { name: 'Supprimer cette ligne', exact: true }).click()
  await panel.getByRole('radio', { name: 'Supprimer cette ligne', exact: true }).press('Escape')
  await expect(panel).toBeHidden()
  await expect(widget.locator('tr')).toHaveCount(2)
  await settings.click()
  await panel.getByRole('radio', { name: 'Ajouter une ligne au-dessous', exact: true }).click()
  await page.keyboard.press('ArrowLeft')
  await expect(
    panel.getByRole('radio', { name: 'Ajouter une ligne au-dessus', exact: true }),
  ).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(widget.locator('tr')).toHaveCount(3)
  await widget.getByRole('button', { name: 'Supprimer · Tableau', exact: true }).click()
  await expect(page.locator('#properties')).toBeVisible()
  await page.locator('#properties').getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect(widget).toHaveCount(1)
})
test('création annulable, dernière ligne/colonne protégées et en-tête facultatif', async ({
  page,
}) => {
  await blank(page)
  await page.getByRole('button', { name: 'Insérer un tableau', exact: true }).click()
  await page
    .locator('#table-properties')
    .getByRole('spinbutton', { name: 'Lignes' })
    .press('Escape')
  await expect(page.locator('[data-editor-widget="tableFigure"]')).toHaveCount(0)
  const widget = await create(page, '1', '1')
  await widget.locator('th').click()
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+Enter' : 'Control+Enter')
  const panel = page.locator('#table-properties')
  await expect(panel).toBeVisible()
  await expect(
    panel.getByRole('radio', { name: 'Supprimer cette ligne', exact: true }),
  ).toBeDisabled()
  await expect(
    panel.getByRole('radio', { name: 'Supprimer cette colonne', exact: true }),
  ).toBeDisabled()
  await panel.getByRole('checkbox', { name: 'Première ligne d’en-tête' }).uncheck()
  await panel.getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(widget.locator('th')).toHaveCount(0)
  await expect(widget.locator('td')).toHaveCount(1)
})

test('un tableau HTML collé reçoit le widget et sa légende, sans habillage imbriqué', async ({
  page,
}) => {
  await blank(page)
  await page.evaluate(() => {
    window.__spike!.editor.commands.insertContent(
      '<table><tbody><tr><td><p>Collé</p></td></tr></tbody></table>',
    )
  })
  const widget = page.locator('[data-editor-widget="tableFigure"]')
  await expect(widget).toHaveCount(1)
  await expect(widget.locator('td')).toContainText('Collé')
  await expect(widget.locator('figcaption')).toHaveCount(1)
  await expect(widget.locator('[data-editor-widget]')).toHaveCount(0)
})

test('un tableau long se répartit sur plusieurs pages et la légende reste éditable', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page, '100', '2')
  await expect(
    widget.getByRole('button', { name: 'Ajouter une ligne en bas du tableau', exact: true }),
  ).toBeDisabled()
  await expect(
    widget.getByRole('button', { name: 'Actions sur la ligne 1', exact: true }),
  ).toBeDisabled()
  await expect
    .poll(() => page.locator('.preview-page').count(), { timeout: 15000 })
    .toBeGreaterThan(1)
  await widget.locator('figcaption').click()
  await page.keyboard.type('Légende')
  await page.keyboard.press('Enter')
  await page.keyboard.type('suite')
  await expect(widget.locator('figcaption')).toHaveText('Légendesuite')
  await expect(widget).toHaveCount(1)
  await widget.getByRole('button', { name: 'Paramètres · Tableau', exact: true }).click()
  const panel = page.locator('#table-properties')
  await expect(
    panel.getByRole('radio', { name: 'Ajouter une ligne au-dessus', exact: true }),
  ).toBeDisabled()
  await panel.getByRole('button', { name: 'Valider', exact: true }).click()
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.state.selection.$from.parent.type.name))
    .toBe('tableCaption')
})

test('une opération confirmée est diffusée une seule fois et son annulation rejoint l’autre fenêtre', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page)
  const peer = await page.context().newPage()
  try {
    await peer.goto('/?client=table-peer')
    await peer.waitForFunction(() => window.__spike?.ready)
    await expect(peer.locator('[data-editor-widget="tableFigure"]')).toHaveCount(1)
    await page.evaluate(() => window.__spike!.provider.flush())
    await peer.evaluate(() => window.__spike!.provider.flush())
    const before = await page.evaluate(() => window.__spike!.provider.revision)
    await widget.locator('th').first().click()
    await widget.getByRole('button', { name: 'Paramètres · Tableau', exact: true }).click()
    const panel = page.locator('#table-properties')
    await panel.getByRole('radio', { name: 'Ajouter une colonne à droite', exact: true }).click()
    await panel.getByRole('button', { name: 'Valider', exact: true }).click()
    await expect(peer.locator('[data-editor-widget="tableFigure"] th')).toHaveCount(3)
    expect(await page.evaluate(() => window.__spike!.provider.revision)).toBe(before + 1)
    await page.keyboard.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z')
    await expect(peer.locator('[data-editor-widget="tableFigure"] th')).toHaveCount(2)
  } finally {
    await peer.close()
  }
})
test('un changement distant entre préparation et publication refuse le lot sans perdre la saisie distante', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page)
  const peer = await page.context().newPage()
  try {
    await peer.goto('/?client=table-race')
    await peer.waitForFunction(() => window.__spike?.ready)
    await expect(peer.locator('[data-editor-widget="tableFigure"]')).toHaveCount(1)
    await peer.evaluate(() => window.__spike!.provider.flush())
    let intervened = false
    await page.route('**/__spike/invoke', async (route) => {
      if (route.request().postDataJSON().command === 'commit_structured_commands' && !intervened) {
        intervened = true
        await peer.evaluate(async () => {
          const { editor, provider } = window.__spike!
          let position = 0
          editor.state.doc.descendants((node, pos) => {
            if (node.type.name === 'tableHeader' && !position) position = pos + 2
          })
          editor.commands.setTextSelection(position)
          editor.commands.insertContent('Édition distante')
          await provider.flush()
        })
      }
      await route.continue()
    })
    await widget.locator('td').first().click()
    await widget.getByRole('button', { name: 'Paramètres · Tableau', exact: true }).click()
    const panel = page.locator('#table-properties')
    await panel.getByRole('radio', { name: 'Ajouter une ligne au-dessous', exact: true }).click()
    await panel.getByRole('button', { name: 'Valider', exact: true }).click()
    await expect(page.locator('#message')).toContainText('Le document a changé')
    await expect(widget.locator('tr')).toHaveCount(2)
    await expect(widget.locator('th').first()).toContainText('Édition distante')
    expect(await page.evaluate(() => window.__spike!.editor.isEditable)).toBe(true)
  } finally {
    await peer.close()
  }
})

test('poignées : bandeau ciblé, conservation des données, clavier et curseur dans la nouvelle zone', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page, '3', '2')
  await widget.locator('th').first().click()
  await page.keyboard.type('Groupe')
  await widget.locator('tr').nth(1).locator('td').first().click()
  await page.keyboard.type('Étudiants')
  await widget.locator('figcaption').click()
  await page.keyboard.type('Une légende à conserver')
  await page.evaluate(() => window.__spike!.provider.flush())
  const handle = widget.getByRole('button', { name: 'Actions sur la ligne 2', exact: true })
  await handle.click()
  const panel = page.locator('#table-properties')
  await expect(panel.getByRole('radio')).toHaveCount(0)
  await expect(panel.getByRole('checkbox')).toHaveCount(0)
  await expect(
    panel.getByRole('button', { name: 'Ajouter une ligne au-dessous', exact: true }),
  ).toBeFocused()
  await expect(widget.locator('tr').nth(1).locator('.table-axis-selected')).toHaveCount(2)
  await panel
    .getByRole('button', { name: 'Ajouter une ligne au-dessous', exact: true })
    .press('Escape')
  await expect(handle).toBeFocused()
  await handle.press('ArrowDown')
  await expect(
    widget.getByRole('button', { name: 'Actions sur la ligne 3', exact: true }),
  ).toBeFocused()
  await page.keyboard.press('Enter')
  await panel.getByRole('button', { name: 'Ajouter une ligne au-dessus', exact: true }).click()
  await expect(widget.locator('tr')).toHaveCount(4)
  await page.keyboard.type('Nouvelle ligne')
  await expect(widget.locator('tr').nth(2).locator('td').first()).toContainText('Nouvelle ligne')
  await expect(widget.locator('tr').nth(1).locator('td').first()).toContainText('Étudiants')
  await widget.getByRole('button', { name: 'Actions sur la colonne 1', exact: true }).click()
  await panel
    .getByRole('button', { name: 'Ajouter une colonne à droite', exact: true })
    .press('Enter')
  await expect(widget.locator('th')).toHaveCount(3)
  await expect(panel).toBeHidden()
  await page.waitForFunction(() => !window.__spike!.provider.historyBusy)
  await expect(page.locator('.tiptap[data-rich-zone-profile="document"]')).toBeFocused()
  await page.keyboard.type('Nouvel en-tête')
  await expect(widget.locator('th').nth(1)).toContainText('Nouvel en-tête')
  await expect(widget.locator('th').first()).toContainText('Groupe')
  await expect(widget.locator('figcaption')).toHaveText('Une légende à conserver')
  // Decorations are absent from the public JSON, so CLI/MCP/exports see only the table.
  const document = await page.evaluate(() => JSON.stringify(window.__spike!.editor.getJSON()))
  expect(document).not.toContain('table-handle')
  expect(document).not.toContain('table-axis-selected')
})

test('les + exécutent une seule commande commune, diffusée et annulable, sans ouvrir de bandeau', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page)
  await widget.locator('figcaption').click()
  await page.keyboard.type('Légende partagée')
  await page.evaluate(() => window.__spike!.provider.clearHistory())
  const peer = await page.context().newPage()
  try {
    await peer.goto('/?client=table-handles-peer')
    await peer.waitForFunction(() => window.__spike?.ready)
    const before = await page.evaluate(() => window.__spike!.provider.revision)
    await widget
      .getByRole('button', { name: 'Ajouter une ligne en bas du tableau', exact: true })
      .click()
    await expect(widget.locator('tr')).toHaveCount(3)
    await expect(page.locator('#table-properties')).toBeHidden()
    await expect(peer.locator('[data-editor-widget="tableFigure"] tr')).toHaveCount(3)
    expect(await page.evaluate(() => window.__spike!.provider.revision)).toBe(before + 1)
    await page.keyboard.type('Dernière ligne')
    await expect(widget.locator('tr').last().locator('td').first()).toContainText('Dernière ligne')
    await page.evaluate(() => window.__spike!.provider.flush())
    await page.keyboard.press('ControlOrMeta+z')
    await expect(widget.locator('tr').last().locator('td').first()).not.toContainText(
      'Dernière ligne',
    )
    await page.keyboard.press('ControlOrMeta+z')
    await expect(peer.locator('[data-editor-widget="tableFigure"] tr')).toHaveCount(2)
    await page.waitForFunction(() => !window.__spike!.provider.historyBusy)
    await expect(page.locator('.tiptap[data-rich-zone-profile="document"]')).toBeFocused()
    await page.keyboard.press('ControlOrMeta+Shift+z')
    await expect(widget.locator('tr')).toHaveCount(3)
    await widget
      .getByRole('button', { name: 'Ajouter une colonne à droite du tableau', exact: true })
      .click()
    await expect(widget.locator('th')).toHaveCount(3)
    await page.waitForFunction(() => !window.__spike!.provider.historyBusy)
    await expect(page.locator('.tiptap[data-rich-zone-profile="document"]')).toBeFocused()
    await page.keyboard.type('Dernière colonne')
    await expect(widget.locator('th').last()).toContainText('Dernière colonne')
    await expect(widget.locator('figcaption')).toHaveText('Légende partagée')
    await expect(peer.locator('[data-editor-widget="tableFigure"] th')).toHaveCount(3)
  } finally {
    await peer.close()
  }
})

test('les contrôles respectent les limites et la déconnexion sans publier de changement', async ({
  page,
}) => {
  await blank(page)
  const widget = await create(page, '2', '12')
  await expect(
    widget.getByRole('button', { name: 'Ajouter une colonne à droite du tableau', exact: true }),
  ).toBeDisabled()
  await expect(
    widget.getByRole('button', { name: 'Actions sur la colonne 1', exact: true }),
  ).toBeDisabled()
  await expect(
    widget.getByRole('button', { name: 'Ajouter une ligne en bas du tableau', exact: true }),
  ).toBeEnabled()
  const before = await page.evaluate(async () => {
    await window.__spike!.provider.disconnect()
    return window.__spike!.editor.getJSON()
  })
  await expect(widget.locator('.table-handle:not(:disabled)')).toHaveCount(0)
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await page.evaluate(() => window.__spike!.provider.reconnect())
  await expect(
    widget.getByRole('button', { name: 'Ajouter une ligne en bas du tableau', exact: true }),
  ).toBeEnabled()
  await expect(
    widget.getByRole('button', { name: 'Ajouter une colonne à droite du tableau', exact: true }),
  ).toBeDisabled()
})

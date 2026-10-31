import { expect, test, type Page } from '@playwright/test'
import { confirmNewDocument } from './document-lifecycle'
import { expectEditorText } from './editor-content'
const html =
  '<p>Avant</p><blockquote><p>Corps</p><cite data-attribution>Source</cite></blockquote><dl><div data-description-item><dt>Terme</dt><dd>Définition</dd></div></dl><figure data-table><table><tbody><tr><td><p>Alpha</p></td><td><p>Bravo</p></td></tr></tbody></table><figcaption data-table-caption>Légende</figcaption></figure><p>Après</p>'
const body = (page: Page) => page.getByRole('textbox', { name: 'Corps du document', exact: true })
async function ready(page: Page, id = 'main') {
  await page.goto('/?client=' + id)
  await page.waitForFunction(() => window.__spike?.ready)
}
async function select(page: Page, first: string, last: string) {
  await page.evaluate(
    ({ first, last }) => {
      const e = window.__spike!.editor
      let from = 0,
        to = 0
      e.state.doc.descendants((node, pos) => {
        if (node.isTextblock) {
          if (node.textContent === first) from = pos + 2
          if (node.textContent === last) to = pos + 3
        }
      })
      e.commands.setTextSelection({ from, to })
      e.view.focus()
    },
    { first, last },
  )
}
async function paste(page: Page, html: string, target?: string) {
  return page.evaluate(
    ({ html, target }) => {
      const dom = target ? document.querySelector(target)! : window.__spike!.editor.view.dom
      const data = new DataTransfer()
      data.setData('text/html', html)
      data.setData('text/plain', 'Texte de repli')
      return dom.dispatchEvent(
        new ClipboardEvent('paste', { bubbles: true, cancelable: true, clipboardData: data }),
      )
    },
    { html, target },
  )
}
test.beforeEach(async ({ page, request }, info) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await ready(page)
  if (info.tags.includes('@header')) return
  await page.evaluate(async (html) => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent(html)
    await provider.flush()
    await provider.clearHistory()
  }, html)
})
test(
  'collage DOM interzones : style conservé, frontières intactes, confirmation et annulation partagées',
  { tag: '@smoke' },
  async ({ page, context }) => {
    const peer = await context.newPage()
    await ready(peer, 'clipboard-peer')
    await select(page, 'Corps', 'Source')
    await paste(page, '<p><strong>Ajout</strong></p><p><em>Suite</em></p>')
    await expectEditorText(body(page).locator('blockquote p'), 'CAjoutSuite')
    await expectEditorText(body(page).locator('blockquote cite'), 'urce')
    await expectEditorText(body(page).locator('blockquote strong'), 'Ajout')
    await expectEditorText(body(peer).locator('blockquote p'), 'CAjoutSuite')
    expect(
      await page.evaluate(async () => {
        await window.__spike!.provider.flush()
        return window.__spike!.provider.historyState.undo_steps
      }),
    ).toBe(1)
    await body(page).press('ControlOrMeta+z')
    await expectEditorText(body(peer).locator('blockquote p'), 'Corps')
    await expectEditorText(body(peer).locator('blockquote cite'), 'Source')
    await peer.close()
  },
)
test('coupe DOM conserve les deux champs, remplit le presse-papiers et reste annulable', async ({
  page,
}) => {
  await select(page, 'Terme', 'Définition')
  const copied = await page.evaluate(() => {
    const data = new DataTransfer()
    window.__spike!.editor.view.dom.dispatchEvent(
      new ClipboardEvent('cut', { bubbles: true, cancelable: true, clipboardData: data }),
    )
    return { html: data.getData('text/html'), text: data.getData('text/plain') }
  })
  expect(copied.text).toContain('erme')
  expect(copied.text).toContain('Dé')
  expect(copied.html).toContain('data-description-item')
  await expectEditorText(body(page).locator('dt'), 'T')
  await expectEditorText(body(page).locator('dd'), 'finition')
  await page.evaluate(() => window.__spike!.provider.flush())
  await body(page).press('ControlOrMeta+z')
  await expectEditorText(body(page).locator('dt'), 'Terme')
  await expectEditorText(body(page).locator('dd'), 'Définition')
})
test('saisie et suppression entre cellules conservent la grille ; une sélection rectangulaire reste native', async ({
  page,
}) => {
  await select(page, 'Alpha', 'Bravo')
  await page.keyboard.insertText('Remplacement')
  await expectEditorText(body(page).locator('td'), ['ARemplacement', 'avo'])
  await expect(body(page).locator('tr')).toHaveCount(1)
  await page.evaluate(async () => {
    await window.__spike!.provider.flush()
    await window.__spike!.provider.history('undo')
  })
  await expectEditorText(body(page).locator('td'), ['Alpha', 'Bravo'])
  await page.evaluate(() => {
    const e = window.__spike!.editor
    const positions: number[] = []
    e.state.doc.descendants((node, pos) => {
      if (node.type.name === 'tableCell') positions.push(pos)
    })
    e.commands.setCellSelection({ anchorCell: positions[0], headCell: positions[1] })
    e.view.focus()
  })
  await page.keyboard.press('Delete')
  await expectEditorText(body(page).locator('td'), ['', ''])
  await expect(body(page).locator('tr')).toHaveCount(1)
  await expectEditorText(body(page).locator('figcaption'), 'Légende')
})
test('un vrai événement paste de tableau crée une seule figure sans enveloppe imbriquée', async ({
  page,
}) => {
  await body(page).fill('Avant')
  await body(page).press('End')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Enter')
  await paste(page, '<table><tbody><tr><td><p>Collé</p></td></tr></tbody></table>')
  await expect(body(page).locator('[data-editor-widget="tableFigure"]')).toHaveCount(1)
  await expectEditorText(body(page).locator('td'), 'Collé')
  await expect(body(page).locator('figcaption')).toHaveCount(1)
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
})
test(
  'le collage dans un titre riche reste sur une ligne et dans le champ de titre',
  { tag: '@header' },
  async ({ page }) => {
    await page.getByRole('button', { name: 'Fichier', exact: true }).click()
    await page.locator('[data-command="document.new"]').click()
    await page.locator('dialog select').first().selectOption('article')
    await confirmNewDocument(page)
    await page.locator('#metadata-tab-titles').click()
    const title = page.getByRole('textbox', { name: 'Titre', exact: true })
    await title.click()
    await paste(
      page,
      '<p><strong>Titre</strong></p><p>complémentaire</p>',
      '[aria-label="Titre"][contenteditable]',
    )
    await expect(title).toHaveText('Titre complémentaire')
    await expect(title.locator('p')).toHaveCount(1)
    await page.evaluate(() => window.__spike!.provider.flush())
    await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
  },
)
test('une image absente collée garde le brouillon et ne publie aucun morceau du document', async ({
  page,
  context,
}) => {
  const peer = await context.newPage()
  await ready(peer, 'clipboard-invalid-peer')
  await page.evaluate(() => {
    const e = window.__spike!.editor
    e.commands.setTextSelection(1)
    e.view.focus()
  })
  await paste(
    page,
    '<figure data-image="images/absente.svg"><img alt="Absente"><figcaption>Brouillon à conserver</figcaption></figure>',
  )
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeVisible()
  await expect(body(page)).toContainText('Brouillon à conserver')
  await expect(body(peer)).not.toContainText('Brouillon à conserver')
  await expectEditorText(body(peer).locator('blockquote p'), 'Corps')
  await peer.close()
})

test('la composition multizone garde les frontières et la préédition privée ; une annulation retrouve le texte', async ({
  page,
  context,
}) => {
  const peer = await context.newPage()
  await ready(peer, 'clipboard-ime-peer')
  await select(page, 'Corps', 'Source')
  await body(page).dispatchEvent('compositionstart', { data: '' })
  await page.evaluate(() => window.__spike!.editor.commands.insertContent('にほん'))
  await expectEditorText(body(page).locator('blockquote p'), 'Cにほん')
  await expectEditorText(body(peer).locator('blockquote p'), 'Corps')
  await expectEditorText(body(page).locator('blockquote cite'), 'urce')
  await body(page).dispatchEvent('compositionend', { data: 'にほん' })
  await expectEditorText(body(peer).locator('blockquote p'), 'Cにほん')
  await page.evaluate(() => window.__spike!.provider.flush())
  await body(page).press('ControlOrMeta+z')
  await expectEditorText(body(peer).locator('blockquote p'), 'Corps')
  await select(page, 'Corps', 'Source')
  await body(page).dispatchEvent('compositionstart', { data: '' })
  await body(page).dispatchEvent('compositionend', { data: '' })
  await expect.poll(() => page.evaluate(() => window.__spike!.provider.composing)).toBe(false)
  await page.evaluate(() => window.__spike!.provider.flush())
  await expectEditorText(body(page).locator('blockquote p'), 'Corps')
  await expectEditorText(body(page).locator('blockquote cite'), 'Source')
  await expectEditorText(body(peer).locator('blockquote cite'), 'Source')
  await peer.close()
})

test('un déplacement DOM interzones conserve le widget source et une seule annulation', async ({
  page,
}) => {
  await select(page, 'Corps', 'Source')
  await page.evaluate(() => {
    const e = window.__spike!.editor,
      data = new DataTransfer()
    e.view.dom.dispatchEvent(
      new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: data }),
    )
    let end = 0
    e.state.doc.descendants((n, pos) => {
      if (n.type.name === 'paragraph' && n.textContent === 'Après') end = pos + 1 + n.content.size
    })
    const point = e.view.coordsAtPos(end)
    e.view.dom.dispatchEvent(
      new DragEvent('drop', {
        bubbles: true,
        cancelable: true,
        dataTransfer: data,
        clientX: point.left,
        clientY: (point.top + point.bottom) / 2,
      }),
    )
  })
  await expectEditorText(body(page).locator('blockquote').first().locator('p'), 'C')
  await expectEditorText(body(page).locator('blockquote').first().locator('cite'), 'urce')
  await page.evaluate(() => window.__spike!.provider.flush())
  await expect(page.getByRole('region', { name: 'Brouillon non synchronisé' })).toBeHidden()
  expect(await page.evaluate(() => window.__spike!.provider.historyState.undo_steps)).toBe(1)
  await body(page).press('ControlOrMeta+z')
  await expect(body(page).locator('blockquote')).toHaveCount(1)
  await expectEditorText(body(page).locator('blockquote p'), 'Corps')
  await expectEditorText(body(page).locator('blockquote cite'), 'Source')
})

test('le collage rectangulaire remplace le contenu sans perdre les rôles d’en-tête ni la légende', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const { editor: e, provider } = window.__spike!
    e.commands.setContent({
      type: 'doc',
      content: [
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
                      type: 'tableHeader',
                      attrs: { scope: 'both' },
                      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Nom' }] }],
                    },
                    {
                      type: 'tableHeader',
                      attrs: { scope: 'column' },
                      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Valeur' }] }],
                    },
                  ],
                },
                {
                  type: 'tableRow',
                  content: [
                    {
                      type: 'tableHeader',
                      attrs: { scope: 'row' },
                      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Ancien' }] }],
                    },
                    {
                      type: 'tableCell',
                      content: [
                        { type: 'paragraph', content: [{ type: 'text', text: 'Contenu' }] },
                      ],
                    },
                  ],
                },
              ],
            },
            { type: 'tableCaption', content: [{ type: 'text', text: 'Légende conservée' }] },
          ],
        },
      ],
    })
    await provider.flush()
    await provider.clearHistory()
    const cells: number[] = []
    e.state.doc.descendants((n, pos) => {
      if (['tableCell', 'tableHeader'].includes(n.type.name)) cells.push(pos)
    })
    e.commands.setCellSelection({ anchorCell: cells[0], headCell: cells[3] })
    e.view.focus()
  })
  await paste(
    page,
    '<table><tr><td><p>A</p></td><td><p>B</p></td></tr><tr><td><p>C</p></td><td><p>D</p></td></tr></table>',
  )
  await expectEditorText(body(page).locator('th'), ['A', 'B', 'C'])
  await expectEditorText(body(page).locator('td'), 'D')
  await expectEditorText(body(page).locator('figcaption'), 'Légende conservée')
  expect(
    await body(page)
      .locator('th')
      .evaluateAll((nodes) => nodes.map((n) => n.getAttribute('data-header-scope'))),
  ).toEqual(['both', 'column', 'row'])
  await page.evaluate(() => window.__spike!.provider.flush())
  await body(page).press('ControlOrMeta+z')
  await expectEditorText(body(page).locator('th'), ['Nom', 'Valeur', 'Ancien'])
  await expectEditorText(body(page).locator('td'), 'Contenu')
})

import { expect, test, type Page } from '@playwright/test'

async function fixture(page: Page, kind: 'orderedList' | 'tableFigure', inside: boolean) {
  await page.request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  return page.evaluate(
    async ({ kind, inside }) => {
      const { editor, provider } = window.__spike!
      const paragraph = (text: string) => ({ type: 'paragraph', content: [{ type: 'text', text }] })
      const cell = (text: string, header = false) => ({
        type: header ? 'tableHeader' : 'tableCell',
        content: [paragraph(text)],
      })
      const widget =
        kind === 'orderedList'
          ? {
              type: kind,
              attrs: { start: 1 },
              content: [
                { type: 'listItem', content: [paragraph('Ligne de liste')] },
                { type: 'listItem', content: [paragraph('Autre item')] },
              ],
            }
          : {
              type: kind,
              content: [
                {
                  type: 'table',
                  content: [
                    {
                      type: 'tableRow',
                      content: [cell('En-tête A', true), cell('En-tête B', true)],
                    },
                    { type: 'tableRow', content: [cell('Nom de ligne'), cell('Valeur')] },
                  ],
                },
                { type: 'tableCaption', content: [{ type: 'text', text: 'Légende conservée' }] },
              ],
            }
      editor.commands.setContent({
        type: 'doc',
        content: [
          ...Array.from({ length: 35 }, (_, i) =>
            paragraph(
              `Paragraphe ${i + 1}. Un texte assez long pour éloigner le widget du début de l’éditeur.`,
            ),
          ),
          widget,
          { type: 'paragraph' },
        ],
      })
      let pos = 1
      if (inside)
        editor.state.doc.descendants((node, at) => {
          if (
            node.type.name === 'paragraph' &&
            node.textContent === (kind === 'orderedList' ? 'Ligne de liste' : 'Nom de ligne')
          )
            pos = at + 6
        })
      editor.commands.setTextSelection(pos)
      editor.view.focus()
      await provider.flush()
      return {
        from: editor.state.selection.from,
        text: editor.state.selection.$from.parent.textContent,
        offset: editor.state.selection.$from.parentOffset,
      }
    },
    { kind, inside },
  )
}
async function returned(page: Page, kind: string) {
  await expect(page.locator('.tiptap[data-rich-zone-profile="document"]')).toBeFocused()
  return page.evaluate((kind) => {
    const editor = window.__spike!.editor,
      selection = editor.state.selection
    const viewport = document.querySelector<HTMLElement>('#editor-scroll')!
    const rect = viewport.getBoundingClientRect(),
      caret = editor.view.coordsAtPos(selection.head)
    let inWidget = false
    for (let d = 1; d <= selection.$from.depth; d++)
      if (selection.$from.node(d).type.name === kind) inWidget = true
    return {
      visible: caret.top >= rect.top && caret.bottom <= rect.bottom,
      scrolled: viewport.scrollTop > 500,
      inWidget,
      from: selection.from,
      text: selection.$from.parent.textContent,
      offset: selection.$from.parentOffset,
    }
  }, kind)
}
for (const kind of ['orderedList', 'tableFigure'] as const)
  for (const inside of [false, true]) {
    test(`${kind} : valider et annuler les préférences rend le contexte au widget, curseur ${inside ? 'dedans' : 'ailleurs'}`, async ({
      page,
    }) => {
      const before = await fixture(page, kind, inside)
      const widget = page.locator(`[data-editor-widget="${kind}"]`)
      const panel = page.locator(kind === 'orderedList' ? '#properties' : '#table-properties')
      const settings = widget.getByRole('button', {
        name: kind === 'orderedList' ? 'Paramètres · Liste numérotée' : 'Paramètres · Tableau',
        exact: true,
      })
      await settings.click()
      if (kind === 'orderedList')
        await panel.getByRole('spinbutton', { name: 'Numéro initial', exact: true }).fill('3')
      else
        await panel
          .getByRole('checkbox', { name: 'Première colonne : en-têtes de lignes', exact: true })
          .check()
      await panel.getByRole('button', { name: 'Valider', exact: true }).click()
      await expect(panel).toBeHidden()
      await page.waitForFunction(() => !window.__spike!.provider.historyBusy)
      await expect
        .poll(() => returned(page, kind))
        .toMatchObject({ visible: true, scrolled: true, inWidget: true, ...(inside ? before : {}) })
      const after = await returned(page, kind)
      await settings.click()
      await page.keyboard.press('Escape')
      await expect(panel).toBeHidden()
      await expect.poll(() => returned(page, kind)).toMatchObject(after)
      await settings.click()
      await panel.getByRole('button', { name: 'Valider', exact: true }).click()
      await expect(panel).toBeHidden()
      await expect.poll(() => returned(page, kind)).toMatchObject(after)
    })
  }

for (const kind of ['footnote', 'link'] as const) {
  test(`${kind} : valider le bandeau en bas du document conserve la position exacte du curseur`, async ({
    page,
  }) => {
    await fixture(page, 'orderedList', false)
    const before = await page.evaluate(async (kind) => {
      const { editor, provider } = window.__spike!
      const tail = editor.state.doc.content.size - 1
      editor.commands.insertContentAt(tail, [
        { type: 'text', text: 'Texte ' },
        ...(kind === 'footnote'
          ? [{ type: 'footnote', attrs: { text: 'Ancienne note' } }]
          : [
              {
                type: 'text',
                text: 'Lien utile',
                marks: [{ type: 'link', attrs: { href: 'https://example.org' } }],
              },
            ]),
      ])
      let pos = 0
      editor.state.doc.descendants((node, at) => {
        if (kind === 'footnote' && node.type.name === 'footnote') pos = at + 1
        if (kind === 'link' && node.isText && node.marks.some((mark) => mark.type.name === 'link'))
          pos = at + 2
      })
      editor.commands.setTextSelection(pos)
      editor.view.focus()
      await provider.flush()
      return pos
    }, kind)
    await page.keyboard.press('Control+Enter')
    const panel = page.locator('#properties')
    await expect(panel).toBeVisible()
    await panel
      .getByRole('textbox', {
        name: kind === 'footnote' ? 'Texte de la note' : 'Adresse du lien',
        exact: true,
      })
      .fill(kind === 'footnote' ? 'Nouvelle note' : 'https://example.org/relecture')
    await panel.getByRole('button', { name: 'Valider', exact: true }).click()
    await expect(panel).toBeHidden()
    await expect
      .poll(() => returned(page, 'paragraph'))
      .toMatchObject({ from: before, visible: true, scrolled: true })
  })
}

test('changer le type de liste conserve le point de rédaction dans le contenu conservé', async ({
  page,
}) => {
  const before = await fixture(page, 'orderedList', true)
  await page.getByRole('button', { name: 'Paramètres · Liste numérotée', exact: true }).click()
  const panel = page.locator('#properties')
  await panel.getByRole('radio', { name: 'Liste à puces', exact: true }).click()
  await panel.getByRole('button', { name: 'Valider', exact: true }).click()
  await expect(panel).toBeHidden()
  await expect
    .poll(() => returned(page, 'bulletList'))
    .toMatchObject({ ...before, visible: true, inWidget: true, scrolled: true })
})

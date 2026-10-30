import { confirmNewDocument } from './document-lifecycle'
import { recordSelection } from './selection-diagnostic'
import { expect, test } from '@playwright/test'

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
})

test('un titre dans une citation est indisponible dans la barre, la palette et au clavier', async ({
  page,
}) => {
  await page.evaluate(async () => {
    window.__spike!.editor.commands.setContent('<blockquote><p>Texte cité</p></blockquote>')
    await window.__spike!.provider.flush()
  })
  await page.locator('[data-editor-widget="quote"] blockquote p').first().click()
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  await expect(page.locator('[data-command="block.heading_1"]')).toBeDisabled()
  await page.keyboard.press('ControlOrMeta+Alt+1')
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
  await page.keyboard.press('ControlOrMeta+Shift+P')
  const input = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
  await input.fill('Titre 1')
  await expect(page.getByRole('option', { name: 'Titre 1', exact: true })).toBeDisabled()
  await input.press('Enter')
  await expect(input).toBeFocused()
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
})

test(
  'la palette agit dans le titre riche actif et ne modifie jamais le corps',
  { tag: '@smoke' },
  async ({ page }, info) => {
    await page.getByRole('button', { name: 'Fichier', exact: true }).click()
    await page.locator('[data-command="document.new"]').click()
    await page.locator('dialog select').first().selectOption('article')
    await confirmNewDocument(page)
    await page.locator('#metadata-tab-titles').click()
    const title = page.getByRole('textbox', { name: 'Titre', exact: true })
    const diagnostic = process.env.TOG_SELECTION_DIAGNOSTIC
      ? await recordSelection(page, 'Titre')
      : undefined
    await title.fill('Titre à relire')
    await title.press('ControlOrMeta+a')
    const before = await page.evaluate(() => window.__spike!.editor.getJSON())
    await expect(page.locator('[data-command="block.quote"]')).toBeDisabled()
    await title.press('ControlOrMeta+Shift+P')
    const input = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
    await input.fill('Gras')
    await input.press('Enter')
    await expect(title.locator('strong')).toHaveText('Titre à relire')
    try {
      await expect(title).toBeFocused()
    } finally {
      await diagnostic?.(info)
    }
    expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
    await page.evaluate(() => window.__spike!.provider.flush())
    await expect
      .poll(() =>
        page.evaluate(
          () => window.__spike!.provider.historyState.availability?.['insert.footnote'].reason,
        ),
      )
      .toBe('body_required')
  },
)

test('la palette ne détourne pas une commande de texte vers le corps depuis un formulaire natif', async ({
  page,
}) => {
  await page.locator('#metadata-tab-header').click()
  const subject = page.getByLabel('Objet', { exact: true })
  await subject.focus()
  const before = await page.evaluate(() => window.__spike!.editor.getJSON())
  await subject.press('ControlOrMeta+Shift+P')
  const input = page.getByRole('combobox', { name: 'Rechercher une commande', exact: true })
  await input.fill('Gras')
  await expect(page.getByRole('option', { name: 'Gras', exact: true })).toBeDisabled()
  await input.press('Enter')
  expect(await page.evaluate(() => window.__spike!.editor.getJSON())).toEqual(before)
})

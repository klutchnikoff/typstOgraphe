import { expect, test, type Page } from '@playwright/test'
async function ready(page: Page) {
  await page.request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await writing(page, 'Original one', 'Original two')
}
async function writing(page: Page, a: string, b: string) {
  await page.evaluate(
    async ({ a, b }) => {
      const { editor, provider } = window.__spike!
      editor.commands.setContent({
        type: 'doc',
        content: [
          { type: 'paragraph', content: [{ type: 'text', text: a }] },
          { type: 'paragraph', content: [{ type: 'text', text: 'Stable anchor' }] },
          { type: 'paragraph', content: [{ type: 'text', text: b }] },
          { type: 'paragraph' },
        ],
      })
      await provider.flush()
    },
    { a, b },
  )
}
async function capture(page: Page, name: string) {
  await page.getByRole('button', { name: 'Créer une version', exact: true }).click()
  await page.locator('dialog [name=name]').fill(name)
  await page.locator('dialog [name=name]').press('Enter')
  await expect(page.locator('dialog[open]')).toHaveCount(0)
  await expect(
    page.locator('.versions-groups [data-version]').filter({ hasText: name }),
  ).toBeVisible()
}
test('Versions: guarded local reprise, immutable right column, real independent previews and external edits', async ({
  page,
  context,
}) => {
  await ready(page)
  await page.locator('.side-rail [data-command="ui.versions"]').click()
  await capture(page, 'Base de travail')
  await writing(page, 'Current one', 'Current two')
  await page.locator('.versions-groups [data-version]').click()
  const dialog = page.locator('.version-dialog[open]')
  await dialog.getByRole('tab', { name: 'Comparer', exact: true }).click()
  await expect(dialog.locator('.comparison-group')).toHaveCount(2)
  await dialog
    .locator('[data-group="0"]')
    .getByRole('button', { name: 'Reprendre ce passage de la version', exact: true })
    .click()
  await expect(dialog.locator('[data-group="0"]')).toContainText('Repris dans le document')
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain('Original one')
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain('Current two')
  await expect(dialog.locator('[data-group="0"] .comparison-side').nth(1)).toHaveText('Current one')
  await expect(dialog.locator('[data-group="1"] button.primary')).toBeEnabled()
  await expect(dialog.locator('.comparison-notice')).toBeHidden()
  await dialog.getByRole('button', { name: 'Aperçus des deux états', exact: true }).click()
  await expect(dialog.locator('.comparison-previews .version-page')).toHaveCount(2)
  await expect
    .poll(() =>
      dialog
        .locator('.comparison-previews img')
        .evaluateAll((images) =>
          images.every(
            (image) =>
              (image as HTMLImageElement).complete && (image as HTMLImageElement).naturalWidth > 0,
          ),
        ),
    )
    .toBe(true)
  await dialog.getByRole('button', { name: 'Voir les changements', exact: true }).click()
  await expect(dialog.locator('[data-group="1"] button.primary')).toBeEnabled()
  await page.evaluate(() => window.__spike!.provider.setSynchronization(true))
  const other = await context.newPage()
  await other.goto('/?client=other')
  await other.waitForFunction(() => window.__spike?.ready)
  await other.evaluate(async () => {
    window.__spike!.editor.commands.insertContentAt(1, 'External ')
    await window.__spike!.provider.flush()
  })
  await expect(dialog.locator('.comparison-notice')).toBeVisible()
  await expect(dialog.locator('[data-group="1"] button.primary')).toBeDisabled()
  await expect(dialog.locator('[data-group="0"] .comparison-side').nth(1)).toHaveText('Current one')
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).press('Escape')
  await expect(dialog).toHaveCount(0)
  await expect(page.locator('.versions-groups [data-version]')).toBeFocused()
  await other.close()
})
test('historical versions stay read only and keyboard tabs preserve a frozen comparison', async ({
  page,
}) => {
  await ready(page)
  await page.locator('.side-rail [data-command="ui.versions"]').click()
  await capture(page, 'Before')
  await writing(page, 'Current one', 'Current two')
  await capture(page, 'After')
  const rows = page.locator('.versions-groups [data-version]')
  await rows.filter({ hasText: 'Before' }).click()
  const dialog = page.locator('.version-dialog[open]')
  await dialog.getByRole('tab', { name: 'Contenu', exact: true }).focus()
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('ArrowRight')
  await expect(dialog.getByRole('tab', { name: 'Comparer', exact: true })).toHaveAttribute(
    'aria-selected',
    'true',
  )
  await expect(dialog.locator('.comparison')).toBeVisible()
  await dialog.locator('.comparison-picker select').selectOption({ label: 'After' })
  await expect(dialog).toContainText('Consultation seule')
  await expect(dialog.locator('.comparison-decisions button')).toHaveCount(0)
  await dialog.locator('.comparison-picker select').selectOption('')
  await expect(dialog.locator('.comparison-decisions button.primary')).toHaveCount(2)
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
})
test('MCP proposals use the same modal, partial decisions and human-only publication', async ({
  page,
}) => {
  await ready(page)
  await page.evaluate(async () => {
    const { provider } = window.__spike!,
      transport = provider.transport
    await transport.invoke('diagnostic_agent_authorize')
    const document = {
      id: provider.generation,
      guard: { document: provider.generation, revision: provider.revision },
    }
    const submitted = await transport.invoke<{ outcome: { id: string } }>('diagnostic_agent', {
      method: 'proposals.submit',
      params: {
        document,
        request_id: 'browser-local',
        summary: 'Two revisions',
        steps: [0, 2].map((i) => ({
          id: 'text.insert',
          params: {
            target: { zone: { kind: 'body', path: [i] }, from: 0, to: 0 },
            text: 'Proposed ',
          },
        })),
      },
    })
    const { proposalDialog } = await import('../../src/version-dialog.ts')
    const { comparisonAPI } = await import('../../src/comparison-api.ts')
    const catalogue =
      await transport.invoke<import('../../src/shell').Catalogue>('content_catalogue')
    proposalDialog({
      id: submitted.outcome.id,
      title: 'Two revisions',
      api: comparisonAPI(provider, transport),
      catalogue,
      returnFocus: () => undefined,
    })
  })
  const dialog = page.locator('.version-dialog[open]')
  await expect(dialog.locator('.comparison-group')).toHaveCount(2)
  await dialog
    .locator('[data-group="0"]')
    .getByRole('button', { name: 'Appliquer cette modification', exact: true })
    .click()
  await expect(dialog.locator('[data-group="0"]')).toContainText('Appliqué au document')
  await expect(dialog.locator('[data-group="1"] button.primary')).toBeEnabled()
  await dialog
    .locator('[data-group="1"]')
    .getByRole('button', { name: 'Refuser', exact: true })
    .click()
  await expect(dialog.locator('[data-group="1"]')).toContainText('Refusé')
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain(
    'Proposed Original one',
  )
  expect(await page.evaluate(() => window.__spike!.editor.getText())).not.toContain(
    'Proposed Original two',
  )
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
  await page.locator('.writing .tiptap').click()
  await page.keyboard.press('ControlOrMeta+z')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.getText()))
    .not.toContain('Proposed Original one')
})

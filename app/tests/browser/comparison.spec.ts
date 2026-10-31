import { expect, test, type Page } from '@playwright/test'
import articleBadges from '../../../fixtures/comparison-article-badges.json' with { type: 'json' }
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
  const initial = await page.evaluate(() => window.__spike!.editor.getText())
  await dialog.locator('[data-group="0"] [data-side="before"]').click()
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toBe(initial)
  await expect(dialog.locator('[data-group="0"] [data-side="before"]')).toHaveAttribute(
    'aria-checked',
    'true',
  )
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
  await expect(
    dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }),
  ).toBeEnabled()
  await page.evaluate(() => window.__spike!.provider.setSynchronization(true))
  const other = await context.newPage()
  await other.goto('/?client=other')
  await other.waitForFunction(() => window.__spike?.ready)
  await other.evaluate(async () => {
    window.__spike!.editor.commands.insertContentAt(1, 'External ')
    await window.__spike!.provider.flush()
  })
  await expect(dialog.locator('.comparison-notice')).toBeVisible()
  await expect(
    dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }),
  ).toBeDisabled()
  await expect(dialog.locator('[data-group="0"] .comparison-side').nth(1)).toHaveText('Current one')
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).press('Escape')
  await expect(dialog).toHaveCount(0)
  await expect(page.locator('.versions-groups [data-version]')).toBeFocused()
  await other.close()
})
test('historical versions compose once and keyboard tabs preserve frozen states', async ({
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
  await expect(dialog.locator('[role=radio]')).toHaveCount(4)
  await dialog.locator('[data-group="0"] [data-side="before"]').click()
  await dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }).click()
  await expect(dialog).toContainText('Choix appliqués et document enregistré.')
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain('Original one')
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain('Current two')
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
})
test('neighboring paragraph edits can be resumed and refused independently', async ({ page }) => {
  await ready(page)
  await page.locator('.side-rail [data-command="ui.versions"]').click()
  await capture(page, 'Paragraphes de départ')
  await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.insertContentAt(editor.state.doc.child(0).nodeSize + 1, 'New ')
    editor.commands.insertContentAt(1, 'New ')
    await provider.flush()
  })
  await page.locator('.versions-groups [data-version]').click()
  const dialog = page.locator('.version-dialog[open]')
  await dialog.getByRole('tab', { name: 'Comparer', exact: true }).click()
  await expect(dialog.locator('.comparison-group')).toHaveCount(2)
  await expect(dialog.locator('[data-group="0"] .comparison-side').first()).toHaveText(
    'Original one',
  )
  await expect(dialog.locator('[data-group="1"] .comparison-side').first()).toHaveText(
    'Stable anchor',
  )
  await dialog.locator('[data-group="0"] [data-side="before"]').click()
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain('New Original one')
  await expect(dialog.locator('[data-group="1"] [data-side="after"]')).toHaveAttribute(
    'aria-checked',
    'true',
  )
  await dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }).click()
  await expect(dialog).toContainText('Choix appliqués et document enregistré.')
  const paragraphs = await page.evaluate(() => {
    const nodes: string[] = []
    window.__spike!.editor.state.doc.forEach((node) => nodes.push(node.textContent))
    return nodes
  })
  expect(paragraphs.slice(0, 2)).toEqual(['Original one', 'New Stable anchor'])
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
})
test('keeping every green choice saves without a Yjs change or extra undo step', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await ready(page)
  await page.locator('.side-rail [data-command="ui.versions"]').click()
  await capture(page, 'Before')
  await writing(page, 'Current one', 'Current two')
  const initial = await page.evaluate(async () => {
    const { provider, editor } = window.__spike!
    const context = await provider.transport.invoke<{ undo_steps: number }>('editing_context', {
      generation: provider.generation,
    })
    return { text: editor.getText(), revision: provider.revision, undo: context.undo_steps }
  })
  await page.locator('.versions-groups [data-version]').click()
  const dialog = page.locator('.version-dialog[open]')
  await dialog.getByRole('tab', { name: 'Comparer', exact: true }).click()
  await dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }).click()
  await expect(dialog).toContainText('Choix appliqués et document enregistré.')
  const result = await page.evaluate(async () => {
    const { provider, editor } = window.__spike!
    const context = await provider.transport.invoke<{ undo_steps: number }>('editing_context', {
      generation: provider.generation,
    })
    return { text: editor.getText(), revision: provider.revision, undo: context.undo_steps }
  })
  expect(result).toEqual(initial)
  expect(errors).toEqual([])
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
})
test('Article snapshots show deleted and inserted words beside a citation in light and dark modes', async ({
  page,
}) => {
  await ready(page)
  const setup = await page.evaluate(async () => {
    const { provider } = window.__spike!
    const catalogue =
      await provider.transport.invoke<import('../../src/shell').Catalogue>('content_catalogue')
    const kind = catalogue.types.find((t) => t.id === 'article')!
    const metadata = structuredClone(kind.metadata)
    metadata.fields.references = [
      {
        id: 'demo-books',
        name: 'books.bib',
        contents: '@book{lefevre2021, author={Lefèvre, Camille}, title={Écrire}, year={2021}}',
      },
    ]
    return {
      generation: provider.generation,
      revision: provider.revision,
      settings: { metadata, modelId: kind.models[0].id, template: kind.models[0].source },
    }
  })
  const created = await page.request.post('/__spike/invoke', {
    data: {
      peer: 'main',
      command: 'new_document',
      args: { revision: setup.revision, settings: setup.settings },
    },
  })
  expect((await created.json()).error).toBeUndefined()
  await page.waitForFunction(
    (generation) => window.__spike?.ready && window.__spike.provider.generation !== generation,
    setup.generation,
  )
  const content = async (document: typeof articleBadges.before) =>
    page.evaluate(async (document) => {
      const { editor, provider } = window.__spike!
      editor.commands.setContent(document)
      await provider.flush()
    }, document)
  await content(articleBadges.before)
  await page.locator('.side-rail [data-command="ui.versions"]').click()
  await capture(page, 'Avant')
  await content(articleBadges.after)
  await capture(page, 'Après')
  await page.locator('.versions-groups [data-version]').filter({ hasText: 'Après' }).click()
  const dialog = page.locator('.version-dialog[open]')
  await dialog.getByRole('tab', { name: 'Comparer', exact: true }).click()
  await dialog.locator('.comparison-picker select').selectOption({ label: 'Avant' })
  const before = dialog.locator('.comparison-side').first(),
    after = dialog.locator('.comparison-side').last()
  await expect(before.locator('del')).toHaveText('doit')
  await expect(after.locator('ins')).toHaveText(['peut', 'xx '])
  await expect(before.locator('.citation-ref')).toContainText('Lefèvre, 2021')
  await expect(after.locator('.citation-ref')).toContainText('Lefèvre, 2021')
  await expect(before.locator('em')).toHaveText('suppose')
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme })
    const red = dialog.locator('[data-group="0"] [data-side="before"]')
    const green = dialog.locator('[data-group="0"] [data-side="after"]')
    await expect(green).toHaveAttribute('aria-checked', 'true')
    await red.click()
    await expect(red).toHaveAttribute('aria-checked', 'true')
    await expect(green).toHaveAttribute('aria-checked', 'false')
    const activeColor = await red.evaluate((node) => ({
      background: getComputedStyle(node).backgroundColor,
      color: getComputedStyle(node).color,
    }))
    expect(activeColor.background).not.toBe('rgba(0, 0, 0, 0)')
    expect(activeColor.background).not.toBe(activeColor.color)
    await green.click()
    for (const tag of ['del', 'ins']) {
      const mark = dialog.locator(tag).first()
      await expect(mark).toBeVisible()
      const style = await mark.evaluate((node) => {
        const mark = getComputedStyle(node),
          parent = getComputedStyle(node.parentElement!)
        return {
          color: mark.color,
          background: mark.backgroundColor,
          parent: parent.color,
          decoration: mark.textDecorationLine,
        }
      })
      expect(style.color).not.toBe(style.parent)
      expect(style.background).not.toBe('rgba(0, 0, 0, 0)')
      expect(style.decoration).toContain(tag === 'del' ? 'line-through' : 'underline')
    }
  }
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
        steps: [0, 1].map((i) => ({
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
  await dialog.locator('[data-group="1"] [data-side="before"]').click()
  expect(await page.evaluate(() => window.__spike!.editor.getText())).not.toContain('Proposed ')
  await dialog.getByRole('button', { name: 'Appliquer et enregistrer', exact: true }).click()
  await expect(dialog).toContainText('Choix appliqués et document enregistré.')
  const count = await page.evaluate(async () => {
    const { provider } = window.__spike!
    const index = await provider.transport.invoke<{ versions: unknown[] }>('versions_list', {
      guard: { document: provider.generation, revision: provider.revision },
    })
    return index.versions.length
  })
  expect(count).toBe(0)
  expect(await page.evaluate(() => window.__spike!.editor.getText())).toContain(
    'Proposed Original one',
  )
  expect(await page.evaluate(() => window.__spike!.editor.getText())).not.toContain(
    'Proposed Stable anchor',
  )
  await dialog.getByRole('button', { name: 'Fermer', exact: true }).click()
  await page.locator('.writing .tiptap').click()
  await page.keyboard.press('ControlOrMeta+z')
  await expect
    .poll(() => page.evaluate(() => window.__spike!.editor.getText()))
    .not.toContain('Proposed Original one')
})

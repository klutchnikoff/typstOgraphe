import { expect, test, type Page } from '@playwright/test'

const image = Buffer.from(
  '<svg xmlns="http://www.w3.org/2000/svg" width="47" height="31"><rect width="47" height="31" fill="#14a573"/></svg>',
)
const file = { name: 'new-resource.svg', mimeType: 'image/svg+xml', buffer: image }
const ownerResources = (page: Page) =>
  page.evaluate(async () => {
    const bootstrap = await window.__spike!.provider.transport.invoke<{
      resources: { path: string }[]
    }>('join')
    return bootstrap.resources.map((r) => r.path).sort()
  })
const flush = (page: Page) => page.evaluate(() => window.__spike!.provider.flush())
async function ready(page: Page, client = 'main') {
  await page.goto(`/?client=${client}`)
  await page.waitForFunction(() => window.__spike?.ready)
}
test.beforeEach(async ({ page, request }) => {
  const reset = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await reset.json()).error).toBeUndefined()
  await ready(page)
})
test('une image préparée puis annulée ne rejoint ni le propriétaire ni les autres fenêtres', async ({
  page,
  context,
}) => {
  const before = await ownerResources(page)
  const peer = await context.newPage()
  await ready(peer, 'resource-peer')
  await page.getByRole('button', { name: 'Paramètres · Image', exact: true }).click()
  await page.locator('#image-file').setInputFiles(file)
  await expect(page.locator('#replace-image')).toHaveText(
    'Image remplacée — appliquer pour confirmer',
  )
  expect(await ownerResources(page)).toEqual(before)
  expect(await peer.evaluate(() => [...window.__spike!.provider.resources.keys()].sort())).toEqual(
    before,
  )
  await page.locator('#cancel-properties').click()
  expect(await ownerResources(page)).toEqual(before)
})
test('signature : ajout, suppression, annulation, partage et réouverture utilisent les ressources confirmées', async ({
  page,
  context,
}) => {
  const before = await ownerResources(page)
  const peer = await context.newPage()
  await ready(peer, 'signature-peer')
  await page.locator('#metadata-tab-sender').click()
  await page.getByLabel('Signature du document', { exact: true }).setInputFiles(file)
  await expect.poll(async () => (await ownerResources(page)).length).toBe(before.length + 1)
  await flush(page)
  const path = await page.evaluate(() =>
    window.__spike!.provider.doc.getMap('settings').get('signature'),
  )
  expect(typeof path).toBe('string')
  await expect
    .poll(() =>
      peer.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('signature')),
    )
    .toBe(path)
  expect(
    await peer.evaluate((path) => window.__spike!.provider.resources.has(path as string), path),
  ).toBe(true)
  await page.getByRole('button', { name: 'Sans signature', exact: true }).click()
  await flush(page)
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('signature')),
  ).toBeNull()
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect
    .poll(() =>
      page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('signature')),
    )
    .toBe(path)
  expect(await ownerResources(page)).toContain(path)
  await page.evaluate(() => window.__spike!.save())
  await peer.close()
  await page.reload()
  await page.waitForFunction(() => window.__spike?.ready)
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('signature')),
  ).toBe(path)
})
test('options : refus concurrent explicite, puis validation sans relais brut', async ({
  page,
  context,
}) => {
  await page.locator('#metadata-tab-general').click()
  const peer = await context.newPage()
  await ready(peer, 'option-peer')
  await peer.locator('#metadata-tab-general').click()
  let release!: () => void,
    prepared = false
  const gate = new Promise<void>((resolve) => {
    release = resolve
  })
  await page.route('**/__spike/invoke', async (route) => {
    if (route.request().postDataJSON().command === 'prepare_structured_commands' && !prepared) {
      const response = await route.fetch()
      prepared = true
      await gate
      await route.fulfill({ response })
    } else await route.continue()
  })
  await page.locator('#document-option-paper').selectOption('a5')
  await expect.poll(() => prepared).toBe(true)
  await peer.locator('#document-option-paper').selectOption('us-letter')
  await flush(peer)
  release()
  const select = page.locator('#document-option-paper')
  await expect(select).toHaveAttribute('aria-invalid', 'true')
  await expect(select).toHaveValue('a5')
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('options.paper')),
  ).toBe('us-letter')
  await select.focus()
  await page.keyboard.press('Tab')
  await expect(page.getByRole('button', { name: 'Annuler', exact: true }).last()).toBeFocused()
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('Enter')
  await expect(select).toHaveAttribute('aria-invalid', 'false')
  await flush(page)
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('options.paper')),
  ).toBe('a5')
  await page.getByRole('button', { name: 'Annuler', exact: true }).click()
  await expect
    .poll(() =>
      page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('options.paper')),
    )
    .toBe('us-letter')
})

test('le profil reste global, puis sa signature est incorporée explicitement ou à la création', async ({
  page,
}) => {
  const before = await ownerResources(page)
  await page.getByRole('button', { name: 'Préférences', exact: true }).click()
  await page.getByLabel('Signature par défaut', { exact: true }).setInputFiles(file)
  await page.getByRole('button', { name: 'Enregistrer les préférences', exact: true }).click()
  await expect(page.locator('#message')).toContainText('Préférences enregistrées')
  expect(await ownerResources(page)).toEqual(before)
  const preferences = await page.evaluate(() =>
    window.__spike!.provider.transport.invoke<{ signature: { path: string } }>('preferences_load'),
  )
  await page.locator('#close-side').click()
  await page.locator('#metadata-tab-sender').click()
  await page.getByRole('button', { name: 'Signature des préférences', exact: true }).click()
  await flush(page)
  expect(await ownerResources(page)).toContain(preferences.signature.path)
  await page.getByRole('button', { name: 'Fichier', exact: true }).click()
  await page.locator('[data-command="document.new"]').click()
  const discard = page.getByRole('button', { name: 'Abandonner', exact: true })
  if (await discard.isVisible()) await discard.click()
  const generation = await page.evaluate(() => window.__spike!.provider.generation)
  await Promise.all([
    page.waitForEvent('domcontentloaded'),
    page.getByRole('button', { name: 'Créer', exact: true }).click(),
  ])
  await page.waitForFunction(
    ({ path, generation }) =>
      window.__spike?.ready &&
      window.__spike.provider.generation !== generation &&
      window.__spike.provider.resources.has(path),
    { path: preferences.signature.path, generation },
  )
  await page.evaluate(() => window.__spike!.save())
  expect(await ownerResources(page)).toEqual([preferences.signature.path])
})

test('changer de modèle garde les fragments Article et entre dans l’historique commun', async ({
  page,
  request,
}) => {
  const setup = await page.evaluate(async () => {
    const { provider } = window.__spike!
    await provider.flush()
    const catalogue =
      await provider.transport.invoke<import('../../src/shell').Catalogue>('content_catalogue')
    const kind = catalogue.types.find((t) => t.id === 'article')!
    return {
      generation: provider.generation,
      revision: provider.revision,
      model: kind.models[0],
      metadata: kind.metadata,
    }
  })
  const result = await request.post('/__spike/invoke', {
    data: {
      peer: 'main',
      command: 'new_document',
      args: {
        revision: setup.revision,
        settings: {
          metadata: setup.metadata,
          modelId: 'article.project',
          template: setup.model.source + '\n// project model',
        },
      },
    },
  })
  expect((await result.json()).error).toBeUndefined()
  await page.waitForFunction(
    (generation) => window.__spike?.ready && window.__spike.provider.generation !== generation,
    setup.generation,
  )
  await page.locator('#metadata-tab-titles').click()
  await page.getByRole('textbox', { name: 'Titre', exact: true }).fill('Un titre conservé')
  await flush(page)
  await page.evaluate(async () => {
    const { provider } = window.__spike!
    await provider.clearHistory()
    const root = provider.doc.getMap('settings')
    const collections = root.get('collections') as import('yjs').Map<
      import('yjs').Map<import('yjs').Map<unknown>>
    >
    const record = collections.get('localizations')!.values().next().value!
    ;(window as any).__retainedModel = {
      collections,
      record,
      title: record.get('title'),
      identity: root.get('identity'),
      generation: provider.generation,
    }
  })
  const commands: string[] = []
  page.on('request', (request) => {
    if (request.url().endsWith('/__spike/invoke')) commands.push(request.postDataJSON().command)
  })
  await page.locator('#metadata-tab-general').click()
  await expect(page.locator('#document-option-model')).toHaveValue('embedded')
  await page.locator('#document-option-model').selectOption(setup.model.id)
  await flush(page)
  expect(commands).toContain('commit_structured_commands')
  expect(commands).not.toContain('relay')
  await page.evaluate(() => window.__spike!.provider.history('undo'))
  await expect(page.locator('#document-option-model')).toHaveValue('embedded')
  await page.evaluate(() => window.__spike!.provider.history('redo'))
  await expect(page.locator('#document-option-model')).toHaveValue(setup.model.id)
  expect(
    await page.evaluate(() => {
      const { provider } = window.__spike!,
        before = (window as any).__retainedModel
      return (
        before.collections === provider.doc.getMap('settings').get('collections') &&
        before.record.get('title') === before.title &&
        before.identity === provider.doc.getMap('settings').get('identity') &&
        before.generation === provider.generation
      )
    }),
  ).toBe(true)
  await page.locator('#metadata-tab-titles').click()
  await expect(page.getByRole('textbox', { name: 'Titre', exact: true })).toHaveText(
    'Un titre conservé',
  )
})

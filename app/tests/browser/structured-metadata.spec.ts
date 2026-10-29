import { expect, test, type Page } from '@playwright/test'

async function ready(page: Page, client = 'main') {
  await page.goto(`/?client=${client}`)
  await page.waitForFunction(() => window.__spike?.ready)
  await page.locator('#metadata-tab-header').click()
}
test.beforeEach(async ({ page, request }) => {
  const reset = await request.post('/__spike/invoke', {
    data: { command: 'reset', peer: 'main', args: {} },
  })
  expect((await reset.json()).error).toBeUndefined()
  await ready(page)
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
test('saisie immédiate, valeurs regroupées et sauvegarde attendant la dernière confirmation typée', async ({
  page,
}) => {
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
  const requests = publications(page),
    input = page.getByLabel('Objet', { exact: true })
  await input.fill('Premier texte')
  await expect.poll(() => prepared).toBe(true)
  await input.fill('Texte final 😀')
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('Texte final 😀')
  await page.evaluate(() => {
    ;(window as any).__savedHeader = false
    ;(window as any).__saveHeader = window.__spike!.save().then(() => {
      ;(window as any).__savedHeader = true
    })
  })
  expect(await page.evaluate(() => (window as any).__savedHeader)).toBe(false)
  release()
  await page.evaluate(() => (window as any).__saveHeader)
  expect(
    await page.evaluate(() =>
      window.__spike!.provider.doc.getMap('settings').get('letter.subject'),
    ),
  ).toBe('Texte final 😀')
  expect(requests.filter((request) => request.command === 'relay')).toEqual([])
  const batch = requests
    .filter((request) => request.command === 'commit_structured_commands')
    .flatMap((request) => request.args.request!.commands)
  expect(
    batch.every(
      (command) =>
        command.id === 'metadata.set' && command.params.target.field === 'letter.subject',
    ),
  ).toBe(true)
  expect(batch.at(-1)?.params.value).toBe('Texte final 😀')
})

test('refus concurrent : brouillon intact, sauvegarde bloquée et annulation explicite du champ', async ({
  page,
  context,
}) => {
  const peer = await context.newPage()
  await ready(peer, 'metadata-peer')
  let prepared = false,
    release!: () => void
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
  const input = page.getByLabel('Objet', { exact: true })
  await input.fill('Ma saisie conservée')
  await expect.poll(() => prepared).toBe(true)
  await peer.getByLabel('Objet', { exact: true }).fill('Version de la collègue')
  await peer.evaluate(() => window.__spike!.provider.flush())
  release()
  await expect(input).toHaveAttribute('aria-invalid', 'true')
  await expect(input).toHaveValue('Ma saisie conservée')
  expect(
    await page.evaluate(() =>
      window.__spike!.provider.doc.getMap('settings').get('letter.subject'),
    ),
  ).toBe('Version de la collègue')
  expect(
    await page.evaluate(() =>
      window.__spike!.provider.flush().then(
        () => false,
        () => true,
      ),
    ),
  ).toBe(true)
  expect(await page.evaluate(() => window.__spike!.provider.rejected)).toBeNull()
  await input.press('Escape')
  await expect(page.locator('#metadata-content')).toBeVisible()
  await expect(input).toHaveValue('Version de la collègue')
  await page.evaluate(() => window.__spike!.provider.flush())
  await expect(input).toBeFocused()
  await peer.close()
})

test('préédition des champs natifs : aucune publication avant validation IME', async ({ page }) => {
  const requests = publications(page),
    input = page.getByLabel('Destinataire', { exact: true })
  await input.focus()
  await input.evaluate((element: HTMLInputElement) => {
    element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }))
    element.value = 'にほん'
    element.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing: true }))
  })
  expect(requests.filter((request) => request.command === 'prepare_structured_commands')).toEqual(
    [],
  )
  expect(
    await page.evaluate(() =>
      window.__spike!.provider.flush().then(
        () => false,
        () => true,
      ),
    ),
  ).toBe(true)
  await input.dispatchEvent('compositionend', { data: 'にほん' })
  await page.evaluate(() => window.__spike!.provider.flush())
  expect(
    await page.evaluate(() =>
      window.__spike!.provider.doc.getMap('settings').get('recipient.name'),
    ),
  ).toBe('にほん')
  await expect(input).toBeFocused()
  expect(requests.filter((request) => request.command === 'relay')).toEqual([])
})

test('changer de modèle attend les champs en cours sans reconstruire les métadonnées', async ({
  page,
}) => {
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
  const identity = await page.evaluate(() =>
    window.__spike!.provider.doc.getMap('settings').get('identity'),
  )
  await page.getByLabel('Objet', { exact: true }).fill('Conserver lors du changement de modèle')
  await expect.poll(() => prepared).toBe(true)
  await page.locator('#metadata-tab-general').click()
  const model = page.locator('#document-option-model')
  await model.selectOption(
    (await model.locator('option:not([value="embedded"])').first().getAttribute('value')) as string,
  )
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('identity')),
  ).toBe(identity)
  release()
  await page.evaluate(() => window.__spike!.provider.flush())
  expect(
    await page.evaluate(() => window.__spike!.provider.doc.getMap('settings').get('identity')),
  ).toBe(identity)
  await page.locator('#metadata-tab-header').click()
  await expect(page.getByLabel('Objet', { exact: true })).toHaveValue(
    'Conserver lors du changement de modèle',
  )
})

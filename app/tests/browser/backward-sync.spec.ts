import { expect, test } from '@playwright/test'

test('le retour depuis Typst centre le curseur et repère temporairement le mot sans sélectionner le texte', async ({
  page,
  request,
}) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  const target = await page.evaluate(async () => {
    const { editor, provider } = window.__spike!
    editor.commands.setContent({
      type: 'doc',
      content: Array.from({ length: 55 }, (_, i) => ({
        type: 'paragraph',
        content: [
          {
            type: 'text',
            text: `Passage ${i} : ce repérage permet de retrouver le texte dans la colonne de rédaction.`,
          },
        ],
      })),
    })
    await provider.flush()
    await provider.clearHistory()
    let pos = 0
    editor.state.doc.descendants((node, offset) => {
      if (node.type.name === 'paragraph' && node.textContent.startsWith('Passage 27 :'))
        pos = offset + 1 + node.textContent.indexOf('repérage') + 3
    })
    editor.commands.setTextSelection(1)
    editor.view.focus()
    document.getElementById('editor-scroll')!.scrollTop = 0
    return { pos, revision: provider.revision, content: JSON.stringify(editor.getJSON()) }
  })
  let point: { page: number; point: [number, number] } | null = null
  await expect
    .poll(
      async () => {
        point = await page.evaluate(
          async ({ revision, pos }) =>
            window.__spike!.provider.transport.invoke<{
              page: number
              point: [number, number]
            } | null>('sync_forward', { revision, position: pos }),
          target,
        )
        return point !== null
      },
      { timeout: 15000 },
    )
    .toBe(true)
  // Use a real forward-mapped Typst point to exercise the complete backward path.
  const click = await page.evaluate((point) => {
    const page = document.querySelector<HTMLElement>(`#pages [data-page="${point.page}"]`)!
    const image = page.querySelector<HTMLElement>('canvas,img')!
    const size = page.querySelector('svg')!.viewBox.baseVal
    const scroll = document.getElementById('pages')!
    const before = image.getBoundingClientRect(),
      viewport = scroll.getBoundingClientRect()
    scroll.scrollTop +=
      before.top +
      (point.point[1] * before.width) / size.width -
      (viewport.top + scroll.clientHeight / 2)
    const rect = image.getBoundingClientRect()
    return {
      x: rect.left + (point.point[0] * rect.width) / size.width,
      y: rect.top + (point.point[1] * rect.width) / size.width,
    }
  }, point!)
  await page.mouse.dblclick(click.x, click.y)
  const marker = page.locator('#editor-scroll .sync-location')
  await expect(marker).toHaveText('repérage')
  const state = await page.evaluate(() => {
    const { editor, provider } = window.__spike!,
      scroll = document.getElementById('editor-scroll')!
    const caret = editor.view.coordsAtPos(editor.state.selection.head),
      viewport = scroll.getBoundingClientRect()
    return {
      empty: editor.state.selection.empty,
      centered: Math.abs(
        (caret.top + caret.bottom) / 2 -
          (viewport.top + scroll.clientTop + scroll.clientHeight / 2),
      ),
      revision: provider.revision,
      content: JSON.stringify(editor.getJSON()),
      undo: provider.historyState.undo_steps,
    }
  })
  expect(state.empty).toBe(true)
  expect(state.centered).toBeLessThan(2)
  expect(state.revision).toBe(target.revision)
  expect(state.content).toBe(target.content)
  expect(state.undo).toBe(0)
  await expect(page.getByRole('textbox', { name: 'Corps du document', exact: true })).toBeFocused()
  await expect(marker).toHaveCount(0, { timeout: 3000 })
})

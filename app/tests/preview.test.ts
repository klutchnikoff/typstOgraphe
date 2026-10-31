import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { Editor } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { previewController } from '../src/preview'
import type { Provider, Transport } from '../src/provider'

function deferred() {
  let resolve!: () => void, reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
const originalDecode = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'decode')
let editor: Editor
let decode = vi.fn<() => Promise<void>>()
let provider: Provider
let version: number
let controller: ReturnType<typeof previewController>
let transport: Transport
const result = (revision: number, pages = ['new']) => ({
  revision,
  pages,
  pageSizes: pages.map(() => [100, 200] as [number, number]),
})
beforeEach(() => {
  vi.useFakeTimers()
  document.body.innerHTML = '<div id="pages"><img src="old" alt="Ancien aperçu"></div>'
  editor = new Editor({
    extensions: richZoneExtensions('document'),
    content: '<p>Test du rendu</p>',
  })
  document.body.append(editor.view.dom)
  editor.commands.setTextSelection(1)
  provider = { revision: 1, generation: 'document-a' } as Provider
  version = 1
  decode = vi.fn(() => Promise.resolve())
  Object.defineProperty(HTMLImageElement.prototype, 'decode', { configurable: true, value: decode })
  transport = {
    invoke: vi.fn(async (command) => (command === 'sync_selection' ? [] : null)),
  } as unknown as Transport
  controller = previewController(editor, provider, transport, () => version)
})
afterEach(() => {
  editor.destroy()
  vi.clearAllTimers()
  vi.useRealTimers()
  if (originalDecode) Object.defineProperty(HTMLImageElement.prototype, 'decode', originalDecode)
  else Reflect.deleteProperty(HTMLImageElement.prototype, 'decode')
  document.body.replaceChildren()
})

it('garde tout l’ancien aperçu jusqu’à ce que toutes les pages soient prêtes', async () => {
  const first = deferred(),
    second = deferred()
  decode.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
  const old = document.querySelector('#pages img')
  const pending = controller.show(result(1, ['first', 'second']), 1)
  expect(document.querySelector('#pages img')).toBe(old)
  first.resolve()
  await Promise.resolve()
  expect(document.querySelector('#pages img')).toBe(old)
  second.resolve()
  expect(await pending).toBe(true)
  expect(
    [...document.querySelectorAll<HTMLImageElement>('#pages img')].map((img) =>
      img.getAttribute('src'),
    ),
  ).toEqual(['first', 'second'])
})

it('une erreur de décodage conserve les pages visibles', async () => {
  const old = document.querySelector('#pages img'),
    failure = new Error('Image corrompue')
  decode.mockRejectedValueOnce(failure)
  await expect(controller.show(result(1), 1)).rejects.toBe(failure)
  expect(document.querySelector('#pages img')).toBe(old)
})

it('un ancien décodage ne peut pas remplacer un aperçu plus récent', async () => {
  const first = deferred()
  decode.mockReturnValueOnce(first.promise)
  const pending = controller.show(result(1, ['outdated']), 1)
  version = provider.revision = 2
  controller.invalidate()
  expect(await controller.show(result(2, ['latest']), 2)).toBe(true)
  first.resolve()
  expect(await pending).toBe(false)
  expect(document.querySelector('#pages img')?.getAttribute('src')).toBe('latest')
})

it('invalide aussi un résultat dont le document a été remplacé ou modifié', async () => {
  for (const change of ['version', 'revision', 'generation', 'invalidation']) {
    provider.revision = version = 1
    provider.generation = 'document-a'
    const held = deferred()
    decode.mockReturnValueOnce(held.promise)
    const pending = controller.show(result(1), 1)
    if (change === 'version') version++
    else if (change === 'revision') provider.revision++
    else if (change === 'generation') provider.generation = 'document-b'
    else controller.invalidate()
    held.resolve()
    expect(await pending).toBe(false)
    expect(document.querySelector('#pages img')?.getAttribute('src')).toBe('old')
  }
})

it('une erreur sur un résultat périmé ne remplace pas le statut du nouvel aperçu', async () => {
  const held = deferred()
  decode.mockReturnValueOnce(held.promise)
  const pending = controller.show(result(1), 1)
  version = provider.revision = 2
  expect(await controller.show(result(2, ['latest']), 2)).toBe(true)
  held.reject(new Error('Erreur tardive'))
  expect(await pending).toBe(false)
  expect(document.querySelector('#pages img')?.getAttribute('src')).toBe('latest')
})

it('un retour de navigation d’une ancienne image ne déplace pas le curseur', async () => {
  let finish!: (position: number) => void
  transport.invoke = vi.fn((command) =>
    command === 'sync_backward'
      ? new Promise<number>((resolve) => {
          finish = resolve
        })
      : Promise.resolve(null),
  ) as Transport['invoke']
  await controller.show(result(1), 1)
  document.querySelector('.preview-page')!.dispatchEvent(new MouseEvent('dblclick'))
  version = provider.revision = 2
  await controller.show(result(2, ['latest']), 2)
  const cursor = editor.state.selection.from
  finish(4)
  await Promise.resolve()
  expect(editor.state.selection.from).toBe(cursor)
})

it('libère les URL binaires périmées et conserve celles de l’aperçu visible', async () => {
  const create = vi
    .fn()
    .mockReturnValueOnce('blob:first')
    .mockReturnValueOnce('blob:next')
    .mockReturnValueOnce('blob:stale')
  const revoke = vi.fn()
  vi.stubGlobal(
    'URL',
    Object.assign(class {}, { createObjectURL: create, revokeObjectURL: revoke }),
  )
  const binary = (revision: number) => ({
    revision,
    pages: [
      { encoding: 'svg' as const, width: 0, height: 0, data: new TextEncoder().encode('<svg/>') },
    ],
    pageSizes: [[100, 200] as [number, number]],
  })
  try {
    expect(await controller.show(binary(1), 1)).toBe(true)
    expect(revoke).not.toHaveBeenCalled()
    version = provider.revision = 2
    expect(await controller.show(binary(2), 2)).toBe(true)
    expect(revoke).toHaveBeenCalledWith('blob:first')
    expect(revoke).not.toHaveBeenCalledWith('blob:next')
    const held = deferred()
    decode.mockReturnValueOnce(held.promise)
    const pending = controller.show(binary(2), 2)
    controller.invalidate()
    held.resolve()
    expect(await pending).toBe(false)
    expect(revoke).toHaveBeenCalledWith('blob:stale')
    controller.destroy()
    expect(revoke).toHaveBeenCalledWith('blob:next')
  } finally {
    vi.unstubAllGlobals()
  }
})

it('seule la dernière navigation demandée peut déplacer et repérer le curseur', async () => {
  const finishes: ((pos: number) => void)[] = []
  transport.invoke = vi.fn((command) =>
    command === 'sync_backward'
      ? new Promise<number>((resolve) => finishes.push(resolve))
      : Promise.resolve(null),
  ) as Transport['invoke']
  await controller.show(result(1), 1)
  const page = document.querySelector('.preview-page')!
  page.dispatchEvent(new MouseEvent('dblclick'))
  page.dispatchEvent(new MouseEvent('dblclick'))
  finishes[1](10)
  await Promise.resolve()
  expect(editor.state.selection.from).toBe(10)
  expect(editor.view.dom.querySelector('.sync-location')?.textContent).toBe('rendu')
  finishes[0](2)
  await Promise.resolve()
  expect(editor.state.selection.from).toBe(10)
})

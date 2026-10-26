import { expect, it, vi } from 'vitest'
import { renderQueue } from '../src/render-queue'

const tick = async () => {
  await Promise.resolve()
  await Promise.resolve()
}
it('démarre sans délai et regroupe les demandes du même événement', async () => {
  const render = vi.fn(async () => {})
  const queue = renderQueue(render)
  for (let i = 0; i < 20; i++) queue.request()
  expect(render).not.toHaveBeenCalled()
  await tick()
  expect(render).toHaveBeenCalledTimes(1)
})
it('ne compile jamais en parallèle et ne garde que la dernière demande', async () => {
  let release!: () => void
  let revision = 1
  const seen: number[] = []
  const render = vi.fn(async () => {
    seen.push(revision)
    await new Promise<void>((resolve) => {
      release = resolve
    })
  })
  const queue = renderQueue(render)
  queue.request()
  await tick()
  for (revision = 2; revision <= 10; revision++) queue.request()
  revision = 10
  expect(render).toHaveBeenCalledTimes(1)
  release()
  await tick()
  expect(seen).toEqual([1, 10])
  release()
  await tick()
  expect(render).toHaveBeenCalledTimes(2)
})
it('la pause attend le rendu actif et la reprise ne perd pas la dernière demande', async () => {
  let release!: () => void
  const render = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        release = resolve
      }),
  )
  const queue = renderQueue(render)
  queue.request()
  await tick()
  const paused = queue.pause()
  queue.request()
  release()
  await paused
  await tick()
  expect(render).toHaveBeenCalledTimes(1)
  queue.resume()
  await tick()
  expect(render).toHaveBeenCalledTimes(2)
  release()
})

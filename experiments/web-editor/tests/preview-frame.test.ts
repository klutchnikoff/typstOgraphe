import { expect, it } from 'vitest'
import { decodeFrame } from '../src/preview-frame'

function packet(
  change: (header: any) => void = () => {},
  payload = new Uint8Array([255, 0, 0, 127]),
) {
  const header = {
    preview: { revision: 3, source: 'text', elapsedMs: 1, pageSizes: [[100, 200]] },
    pages: [{ encoding: 'rgba', width: 1, height: 1, offset: 0, length: payload.length }],
  }
  change(header)
  const json = new TextEncoder().encode(JSON.stringify(header))
  const bytes = new Uint8Array(8 + json.length + payload.length)
  bytes.set([84, 79, 71, 49])
  new DataView(bytes.buffer).setUint32(4, json.length, true)
  bytes.set(json, 8)
  bytes.set(payload, 8 + json.length)
  return bytes.buffer
}
it('lit les pixels sans recopier le buffer IPC et conserve l’alpha', () => {
  const buffer = packet()
  const page = decodeFrame(buffer).pages[0]
  if (typeof page === 'string') throw new Error('Page binaire attendue')
  expect(page.data.buffer).toBe(buffer)
  expect([...page.data]).toEqual([255, 0, 0, 127])
})
it('refuse les en-têtes, dimensions, bornes et formats incohérents', () => {
  for (const change of [
    (h: any) => {
      h.pages[0].encoding = 'html'
    },
    (h: any) => {
      h.pages[0].offset = 1
    },
    (h: any) => {
      h.pages[0].length = 100
    },
    (h: any) => {
      h.pages[0].length = 0
    },
    (h: any) => {
      h.pages[0].width = 2
    },
    (h: any) => {
      h.pages[0].height = -1
    },
    (h: any) => {
      h.preview.pageSizes = []
    },
    (h: any) => {
      h.preview.pageSizes = [[0, 200]]
    },
  ])
    expect(() => decodeFrame(packet(change))).toThrow()
  expect(() => decodeFrame(new ArrayBuffer(4))).toThrow()
  const bad = packet()
  new DataView(bad).setUint32(4, 0xffffffff, true)
  expect(() => decodeFrame(bad)).toThrow()
  const magic = packet()
  new DataView(magic).setUint32(0, 0)
  expect(() => decodeFrame(magic)).toThrow()
})
it('accepte un SVG binaire sans décoder son XML dans le DOM', () => {
  const value = decodeFrame(
    packet((h) => {
      h.pages[0].encoding = 'svg'
      h.pages[0].width = h.pages[0].height = 0
    }, new TextEncoder().encode('<svg/>')),
  )
  expect(value.pages[0]).toMatchObject({ encoding: 'svg' })
})

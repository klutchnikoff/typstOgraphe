import type { Transport } from './provider'

export type Encoding = 'png' | 'rgba' | 'svg' | 'dataurl'
export interface BinaryPage {
  encoding: Exclude<Encoding, 'dataurl'>
  width: number
  height: number
  data: Uint8Array<ArrayBuffer>
}
export interface Preview {
  revision: number
  source: string
  pages: (string | BinaryPage)[]
  elapsedMs: number
  timings?: Record<string, number>
  pageSizes?: [number, number][]
}
const invalid = () => new Error('Réponse de rendu binaire invalide.')
/** Views of the native IPC buffer: never JSON byte arrays or base64 in Tauri. */
export function decodeFrame(buffer: ArrayBuffer): Preview {
  if (buffer.byteLength < 8) throw invalid()
  const view = new DataView(buffer)
  if (view.getUint32(0) !== 0x544f4731) throw invalid()
  const start = 8 + view.getUint32(4, true)
  if (start > buffer.byteLength) throw invalid()
  const header = JSON.parse(
    new TextDecoder('utf-8', { fatal: true }).decode(new Uint8Array(buffer, 8, start - 8)),
  )
  const value = header.preview as Preview
  if (
    !Number.isSafeInteger(value?.revision) ||
    typeof value.source !== 'string' ||
    !Number.isFinite(value.elapsedMs) ||
    !Array.isArray(header.pages) ||
    !Array.isArray(value.pageSizes) ||
    value.pageSizes.length !== header.pages.length ||
    !value.pageSizes.every(
      (s) => Array.isArray(s) && s.length === 2 && s.every((n) => Number.isFinite(n) && n > 0),
    )
  )
    throw invalid()
  let offset = 0
  value.pages = header.pages.map(
    (page: { encoding: string; width: number; height: number; offset: number; length: number }) => {
      if (
        !['png', 'rgba', 'svg'].includes(page.encoding) ||
        !Number.isSafeInteger(page.offset) ||
        page.offset !== offset ||
        !Number.isSafeInteger(page.length) ||
        page.length <= 0 ||
        page.length > buffer.byteLength - start - offset ||
        !Number.isSafeInteger(page.width) ||
        page.width < 0 ||
        !Number.isSafeInteger(page.height) ||
        page.height < 0 ||
        (page.encoding === 'rgba' &&
          (page.width === 0 || page.height === 0 || page.length !== page.width * page.height * 4))
      )
        throw invalid()
      const data = new Uint8Array(buffer, start + offset, page.length)
      offset += page.length
      return {
        encoding: page.encoding as BinaryPage['encoding'],
        width: page.width,
        height: page.height,
        data,
      }
    },
  )
  if (offset !== buffer.byteLength - start) throw invalid()
  return value
}
export async function requestPreview(
  transport: Transport,
  snapshot: unknown,
  encoding: Encoding = 'rgba',
) {
  const started = performance.now()
  if (encoding === 'dataurl') {
    const result = await transport.invoke<Preview>('preview', { snapshot })
    return {
      result,
      bytes: new TextEncoder().encode(JSON.stringify(result)).byteLength,
      requestMs: performance.now() - started,
    }
  }
  const buffer = await transport.invoke<ArrayBuffer>('preview_frame', { snapshot, encoding })
  const received = performance.now()
  const result = decodeFrame(buffer)
  return { result, bytes: buffer.byteLength, requestMs: received - started }
}

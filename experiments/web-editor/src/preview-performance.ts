import type { Editor, JSONContent } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import { requestPreview, type Encoding, type Preview } from './preview-frame'

// Native WKWebView + real IPC only. Not a timing test in the HTTP browser driver.
const frame = () =>
  new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error(`Frame suspendue : ${document.visibilityState}`)),
      3000,
    )
    requestAnimationFrame(() => {
      clearTimeout(timeout)
      resolve()
    })
  })
const painted = async () => {
  if (document.visibilityState !== 'visible') throw new Error('Fenêtre de mesure masquée.')
  await frame()
  await frame()
  if (document.visibilityState !== 'visible') throw new Error('Fenêtre masquée pendant la mesure.')
}
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
interface Harness {
  pause: () => Promise<void>
  resume: () => void
  snapshot: () => unknown
  show: (result: Preview) => Promise<boolean>
  observe: (callback: (revision: number) => void) => void
}
export async function nativePreviewPerformance(
  transport: Transport,
  provider: Provider,
  editor: Editor,
  harness: Harness,
) {
  const samples: Record<string, unknown>[] = []
  const typing: Record<string, unknown>[] = []
  const fixture = editor.getJSON()
  const text =
    'Madame, Monsieur, je vous remercie de bien vouloir examiner ce courrier. Les informations demandées seront transmises dès réception des pièces justificatives. Nous restons disponibles pour toute précision complémentaire.'
  const paragraph = (value: string): JSONContent => ({
    type: 'paragraph',
    content: [{ type: 'text', text: value }],
  })
  const workloads: [string, JSONContent][] = [
    ['letter', { type: 'doc', content: [paragraph(text)] }],
    ['fixture', fixture],
    [
      'long',
      {
        type: 'doc',
        content: Array.from({ length: 60 }, (_, i) => paragraph(`${i + 1}. ${text}`)),
      },
    ],
  ]
  try {
    await harness.pause()
    await transport.invoke('diagnostic_event', {
      target: 'main',
      step: 'progress',
      payload: { phase: 'ready', visibility: document.visibilityState },
    })
    // Rotate the order to avoid systematically warming Typst for one transport.
    const encodings: Encoding[] = ['dataurl', 'png', 'rgba', 'svg']
    for (const [workload, content] of workloads) {
      editor.commands.setContent(content)
      await provider.flush()
      for (let round = 0; round < 10; round++) {
        editor.commands.insertContentAt(1, String.fromCharCode(97 + round))
        await provider.flush()
        for (let j = 0; j < encodings.length; j++) {
          const encoding = encodings[(round + j) % encodings.length]
          const start = performance.now()
          const { result, bytes, requestMs } = await requestPreview(
            transport,
            harness.snapshot(),
            encoding,
          )
          const received = performance.now()
          if (round === 0 && j === 0)
            await transport.invoke('diagnostic_event', {
              target: 'main',
              step: 'progress',
              payload: { phase: 'received', workload, encoding },
            })
          if (!(await harness.show(result)))
            throw new Error('Rendu natif périmé pendant le benchmark.')
          const published = performance.now()
          if (round === 0 && j === 0)
            await transport.invoke('diagnostic_event', {
              target: 'main',
              step: 'progress',
              payload: { phase: 'published', workload, encoding },
            })
          await painted()
          samples.push({
            workload,
            round,
            encoding,
            pages: result.pages.length,
            bytes,
            backendMs: result.elapsedMs,
            requestMs,
            prepareMs: published - received,
            frameMs: performance.now() - start,
            timings: result.timings,
          })
        }
      }
      await transport.invoke('diagnostic_event', {
        target: 'main',
        step: 'progress',
        payload: { workload, samples: samples.length },
      })
    }
    // The actual automatic pipeline, including queue, Yjs relay, projection and IPC.
    for (const [workload, content] of workloads.slice(0, 2)) {
      editor.commands.setContent(content)
      await provider.flush()
      let finish: (() => void) | undefined
      harness.observe((revision) => {
        if (revision === provider.revision) finish?.()
      })
      harness.resume()
      for (let i = 0; i < 12; i++) {
        const ready = new Promise<void>((resolve) => {
          finish = resolve
        })
        const start = performance.now()
        editor.commands.insertContentAt(1, String.fromCharCode(65 + i))
        await ready
        await painted()
        typing.push({ workload, mode: 'single', elapsedMs: performance.now() - start })
      }
      let publications = 0
      harness.observe((revision) => {
        if (revision === provider.revision) {
          publications++
          finish?.()
        }
      })
      const burstStarted = performance.now()
      for (let i = 0; i < 15; i++) {
        editor.commands.insertContentAt(1, 'x')
        await wait(40)
      }
      typing.push({
        workload,
        mode: 'burst',
        publications,
        durationMs: performance.now() - burstStarted,
      })
      const start = performance.now()
      const last = new Promise<void>((resolve) => {
        finish = resolve
      })
      editor.commands.insertContentAt(1, 'z')
      await last
      await painted()
      typing.push({ workload, mode: 'burst-tail', elapsedMs: performance.now() - start })
      await harness.pause()
    }
    await transport.invoke('diagnostic_finish', {
      report: {
        ok: true,
        success: true,
        userAgent: navigator.userAgent,
        viewport: {
          width: window.innerWidth,
          height: window.innerHeight,
          devicePixelRatio: window.devicePixelRatio,
        },
        visibility: document.visibilityState,
        defaultEncoding: 'rgba',
        scope:
          'Native IPC; transaction to two requestAnimationFrames after publication, not a hardware presentation timestamp. First round discarded as warm-up; app and fonts already initialized. Synthetic editor transactions, not OS keystrokes.',
        samples,
        typing,
      },
    })
  } catch (error) {
    await transport.invoke('diagnostic_finish', {
      report: { ok: false, success: false, error: String(error), samples, typing },
    })
  }
}

import type { Editor } from '@tiptap/core'
import type { Provider, Transport } from './provider'
import { requestPreview } from './preview-frame'

/** Diagnostic instrumentation only: installed before listeners, never in normal launches. */
export function measureTransport(transport: Transport) {
  let counts: Record<string, number> = {}
  let bytes = 0
  const record = (name: string, value: unknown) => {
    counts[name] = (counts[name] || 0) + 1
    bytes += new TextEncoder().encode(JSON.stringify(value) || '').byteLength
  }
  return {
    reset() {
      counts = {}
      bytes = 0
    },
    snapshot() {
      return { counts: { ...counts }, jsonBytes: bytes }
    },
    transport: {
      async invoke<T>(command: string, args?: Record<string, unknown>) {
        record(`invoke:${command}`, args)
        return transport.invoke<T>(command, args)
      },
      async listen<T>(name: string, handler: (payload: T) => void) {
        return transport.listen<T>(name, (payload) => {
          record(`event:${name}`, payload)
          handler(payload)
        })
      },
    } satisfies Transport,
  }
}
export async function synchronizationPerformance(
  transport: Transport,
  provider: Provider,
  editor: Editor,
  meter: ReturnType<typeof measureTransport>,
  pause: () => Promise<void>,
  snapshot: () => unknown,
  active = false,
) {
  const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
  try {
    await pause()
    if (active) await provider.setSynchronization(true)
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'Courrier administratif pour mesurer la confirmation des éditions.',
            },
          ],
        },
      ],
    })
    await provider.flush()
    await wait(2000)
    meter.reset()
    const idleStarted = performance.now()
    await wait(32000) // Actual elapsed time is recorded: macOS may throttle inactive views.
    const idle = { durationMs: performance.now() - idleStarted, ...meter.snapshot() }
    const latencyMs: number[] = []
    meter.reset()
    for (let i = 0; i < 110; i++) {
      const start = performance.now()
      editor.commands.insertContentAt(1, 'a')
      await provider.flush()
      if (i >= 10) latencyMs.push(performance.now() - start)
      if (i === 9) meter.reset()
    }
    await wait(100)
    const edits = meter.snapshot()
    editor.commands.setTextSelection(editor.state.doc.firstChild!.nodeSize - 1)
    editor.view.focus()
    await wait(200)
    const focused = {
      hasFocus: editor.view.hasFocus(),
      latencyMs: [] as number[],
      counts: {} as Record<string, number>,
      jsonBytes: 0,
      allEditsFocused: true,
    }
    if (focused.hasFocus) {
      meter.reset()
      for (let i = 0; i < 110; i++) {
        focused.allEditsFocused &&= editor.view.hasFocus()
        const start = performance.now()
        editor.commands.insertContent('f')
        await provider.flush()
        if (i >= 10) focused.latencyMs.push(performance.now() - start)
        if (i === 9) meter.reset()
      }
      await wait(100)
      Object.assign(focused, meter.snapshot())
    }
    const typstMs: number[] = []
    for (let i = 0; i < 11; i++) {
      editor.commands.insertContentAt(1, 't')
      await provider.flush()
      const { result } = await requestPreview(transport, snapshot(), 'rgba')
      if (i > 0) typstMs.push(result.elapsedMs)
    }
    await transport.invoke('diagnostic_finish', {
      report: {
        success: true,
        scope:
          'Native WebView, production assets, real Rust validation/history; synthetic transactions, automatic rendering paused. 10 warm-up edits then 100 measured edits without focus, then the same series with focus when available; 10 separate warm incremental Typst renders; bytes are serialized outgoing arguments and received event payloads, not wire overhead or process memory.',
        active,
        focused,
        typstMs,
        userAgent: navigator.userAgent,
        idle,
        edits,
        latencyMs,
      },
    })
  } catch (error) {
    await transport.invoke('diagnostic_finish', {
      report: { success: false, error: String(error) },
    })
  }
}

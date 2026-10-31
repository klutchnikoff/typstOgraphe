import { afterEach, expect, it, vi } from 'vitest'
import { Editor } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { mountAssistance } from '../src/assistance'
import type { Provider, Transport } from '../src/provider'
const originalShowModal = HTMLDialogElement.prototype.showModal,
  originalClose = HTMLDialogElement.prototype.close

let editor: Editor
let cleanup: (() => void) | undefined
afterEach(() => {
  cleanup?.()
  editor?.destroy()
  document.body.replaceChildren()
  HTMLDialogElement.prototype.showModal = originalShowModal
  HTMLDialogElement.prototype.close = originalClose
  vi.restoreAllMocks()
})
it.each(['review', 'ime'] as const)(
  'assistance %s opens the shared comparator and never sends document content for human approval',
  async (mode) => {
    document.body.innerHTML =
      '<nav class="side-rail"></nav><aside class="side-panel"><div id="content"></div></aside>'
    HTMLDialogElement.prototype.showModal = function () {
      this.setAttribute('open', '')
    }
    HTMLDialogElement.prototype.close = function () {
      this.removeAttribute('open')
    }
    editor = new Editor({ extensions: richZoneExtensions('document'), content: '<p>Base</p>' })
    await vi.waitFor(() => expect(editor.getJSON().content).toHaveLength(2))
    const original = editor.getJSON(),
      message = vi.fn()
    const invoke = vi.fn(async (command: string) => {
      if (command === 'agent_state')
        return {
          connected: true,
          proposal: {
            id: 'trusted',
            revision: 4,
            status: 'pending',
            summary: 'Proposition',
            before: original,
            after: original,
          },
        }
      if (command === 'content_catalogue') return { locale: 'fr', types: [] }
      if (command === 'comparison_open')
        return {
          id: 'review',
          before: {
            state: {
              document: original,
              settings: { metadata: { type: 'courrier', fields: { options: {} } } },
            },
            resources: [],
          },
          after: {
            state: {
              document: original,
              settings: { metadata: { type: 'courrier', fields: { options: {} } } },
            },
            resources: [],
          },
          report: {
            guard: { document: 'doc', revision: 4 },
            source: 'after',
            context: 'proposal',
            stale: false,
            groups: [],
          },
        }
    })
    const provider = { generation: 'doc', revision: 4, flush: vi.fn(async () => {}) }
    if (mode === 'ime')
      Object.defineProperty(editor.view, 'composing', { value: true, configurable: true })
    mountAssistance(
      provider as unknown as Provider,
      { invoke, listen: async () => () => {} } as unknown as Transport,
      editor,
      () => {
        const content = document.querySelector<HTMLElement>('#content')!
        content.replaceChildren()
        return content
      },
      message,
      true,
    )
    document.querySelector<HTMLButtonElement>('.side-rail button')!.click()
    await vi.waitFor(() => expect(document.querySelector('button.primary')).not.toBeNull())
    document.querySelector<HTMLButtonElement>('button.primary')!.click()
    if (mode === 'ime') {
      await vi.waitFor(() => expect(message).toHaveBeenCalled())
      expect(invoke.mock.calls.some(([c]) => c === 'comparison_open')).toBe(false)
    } else {
      await vi.waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('comparison_open', {
          guard: { document: 'doc', revision: 4 },
          origin: { kind: 'proposal', id: 'trusted' },
        }),
      )
      expect(document.querySelector('.comparison')).not.toBeNull()
    }
    expect(invoke.mock.calls.some(([c]) => c === 'agent_accept')).toBe(false)
    expect(editor.getJSON()).toEqual(original)
  },
)

it('never replaces another active sidebar panel when an agent notification arrives', async () => {
  document.body.innerHTML =
    '<nav class="side-rail"></nav><aside class="side-panel"><div id="content"></div></aside>'
  editor = new Editor({ extensions: richZoneExtensions('document'), content: '<p>Base</p>' })
  let notify = () => {}
  const invoke = vi.fn(async () => ({ connected: true, proposal: null }))
  const host = document.querySelector<HTMLElement>('#content')!
  mountAssistance(
    {} as Provider,
    {
      invoke,
      listen: async (_event: string, callback: () => void) => {
        notify = callback
        return () => {}
      },
    } as unknown as Transport,
    editor,
    () => {
      host.replaceChildren()
      return host
    },
    vi.fn(),
    true,
  )
  document.querySelector<HTMLButtonElement>('.side-rail button')!.click()
  await vi.waitFor(() => expect(host.querySelector('.assistance-panel')).not.toBeNull())
  host.replaceChildren(document.createTextNode('Versions'))
  const calls = invoke.mock.calls.length
  notify()
  await Promise.resolve()
  expect(host.textContent).toBe('Versions')
  expect(invoke).toHaveBeenCalledTimes(calls)
})

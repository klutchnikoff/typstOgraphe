import { afterEach, expect, it, vi } from 'vitest'
import { Editor } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { mountAssistance } from '../src/assistance'
import type { Provider, Transport } from '../src/provider'
let editor: Editor

afterEach(() => {
  editor?.destroy()
  document.body.replaceChildren()
})

it.each(['accept', 'refused', 'stale', 'ime'] as const)(
  'assistance %s: only the trusted proposal ID reaches native acceptance',
  async (mode) => {
    document.body.innerHTML =
      '<nav class="side-rail"></nav><aside class="side-panel"><div id="content"></div></aside>'
    editor = new Editor({ extensions: richZoneExtensions('document'), content: '<p>Base</p>' })
    await vi.waitFor(() => expect(editor.getJSON().content).toHaveLength(2))
    const original = editor.getJSON()
    const message = vi.fn()
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
            after: {
              type: 'doc',
              content: [{ type: 'paragraph', content: [{ type: 'text', text: 'Proposal' }] }],
            },
          },
        }
      if (command === 'agent_accept' && mode === 'refused') throw new Error('stale_revision')
    })
    const provider = {
      revision: 4,
      flush: vi.fn(async () => {
        if (mode === 'stale') provider.revision = 5
      }),
      imageURL: () => '',
    }
    if (mode === 'ime')
      Object.defineProperty(editor.view, 'composing', { value: true, configurable: true })
    const transport = { invoke, listen: async () => () => {} } as unknown as Transport
    mountAssistance(
      provider as unknown as Provider,
      transport,
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
    await vi.waitFor(() => expect(message).toHaveBeenCalled())
    const calls = invoke.mock.calls.filter(([command]) => command === 'agent_accept')
    if (mode === 'accept' || mode === 'refused')
      expect(calls).toEqual([['agent_accept', { id: 'trusted' }]])
    else expect(calls).toHaveLength(0)
    expect(editor.getJSON()).toEqual(original)
  },
)

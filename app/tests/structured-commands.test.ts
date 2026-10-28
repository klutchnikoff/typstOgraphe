import { afterEach, expect, it, vi } from 'vitest'
import type { Editor } from '@tiptap/core'
import * as Y from 'yjs'
import type { Provider, Transport } from '../src/provider'
import { executeStructured } from '../src/structured-commands'
const docs: Y.Doc[] = []
afterEach(() => docs.splice(0).forEach((doc) => doc.destroy()))
function setup() {
  const doc = new Y.Doc()
  docs.push(doc)
  doc.getMap('settings').set('identity', 'one')
  const provider = {
    doc,
    generation: 'one',
    revision: 5,
    flush: vi.fn(async () => {}),
  } as unknown as Provider
  let cursorSame = true
  const editor = {
    isDestroyed: false,
    view: { composing: false },
    state: { selection: { eq: () => cursorSame } },
  } as unknown as Editor
  let complete!: (value: { changed: boolean; update: number[] }) => void
  const invoke = vi.fn(
    () =>
      new Promise<{ changed: boolean; update: number[] }>((resolve) => {
        complete = resolve
      }),
  )
  return {
    provider,
    editor,
    transport: { invoke } as unknown as Transport,
    invoke,
    finish: () => complete({ changed: false, update: [] }),
    move: () => {
      cursorSame = false
    },
  }
}
it('prépare une opération avec les relais acquittés et ne crée aucune édition pour une réponse inchangée', async () => {
  const s = setup(),
    request = executeStructured(s.editor, s.provider, s.transport, [])
  await vi.waitFor(() => expect(s.invoke).toHaveBeenCalledOnce())
  expect(s.provider.flush).toHaveBeenCalledOnce()
  expect(s.invoke.mock.calls[0]).toMatchObject([
    'prepare_structured_commands',
    { request: { generation: 'one', revision: 5, commands: [] } },
  ])
  s.finish()
  await expect(request).resolves.toBe(false)
})
it.each(['text', 'revision', 'generation', 'cursor'] as const)(
  'refuse une réponse préparée après changement de %s sans écraser le travail courant',
  async (change) => {
    const s = setup(),
      request = executeStructured(s.editor, s.provider, s.transport, [])
    await vi.waitFor(() => expect(s.invoke).toHaveBeenCalledOnce())
    if (change === 'text') s.provider.doc.getText('draft').insert(0, 'Mon travail')
    if (change === 'revision') s.provider.revision++
    if (change === 'generation') s.provider.generation = 'two'
    if (change === 'cursor') s.move()
    const before = Y.encodeStateAsUpdate(s.provider.doc)
    s.finish()
    await expect(request).rejects.toThrow('Le document a changé')
    expect(Y.encodeStateAsUpdate(s.provider.doc)).toEqual(before)
  },
)

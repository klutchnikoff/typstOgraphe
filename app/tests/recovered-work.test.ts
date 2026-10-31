import { afterEach, expect, it, vi } from 'vitest'
import { Provider, type Transport } from '../src/provider'
import { consumeRecovered } from '../src/recovered-work'
import { contextPanel } from '../src/context-panel'
import { hydrateForm } from '../src/form-recovery'
import { privateDrafts } from '../src/private-drafts'
import { decideClose } from '../src/close-decision'

HTMLDialogElement.prototype.showModal = function () {
  this.open = true
}
HTMLDialogElement.prototype.close = function () {
  this.open = false
}
afterEach(() => document.body.replaceChildren())
const item = {
  id: 'recovered-author',
  kind: 'form' as const,
  payload: {
    id: 'private',
    form: 'author',
    context: { kind: 'record', collection: 'authors' },
    data: {
      fields: [{ id: 'name', type: 'text', value: 'Private author', checked: false }],
      choices: [],
      rich: [],
      data: null,
    },
  },
}
function setup() {
  const provider = new Provider('main', { invoke: async () => {} } as unknown as Transport)
  provider.recoveredItems = [structuredClone(item)]
  const force = vi.fn(async () => {})
  provider.privateRecovery = { force, flush: force, stop: () => {} }
  return { provider, force }
}
it('opening and cancelling a recovered form never consumes its saved source or publishes it', () => {
  const { provider } = setup(),
    form = contextPanel({
      id: 'author',
      closeLabel: 'Cancel',
      close: () => {},
      canConfirm: () => true,
    })
  const input = document.createElement('input')
  input.id = 'name'
  form.body.append(input)
  document.body.append(form.element)
  form.element.hidden = false
  const applied = vi.fn()
  input.oninput = applied
  form.trackRecovery(provider, { kind: 'record' })
  expect(hydrateForm(provider, form.element, item)).toBe(true)
  expect(input.value).toBe('Private author')
  expect(privateDrafts(provider).snapshot()[0].sourceId).toBe(item.id)
  form.discardRecovery()
  expect(provider.recoveredItems).toEqual([item])
  expect(provider.revision).toBe(0)
  provider.destroy()
})
it('a failed durable acknowledgement keeps the recovered item available', async () => {
  const { provider, force } = setup()
  force.mockRejectedValueOnce(new Error('Disk full'))
  await expect(consumeRecovered(provider, item.id)).rejects.toThrow('Disk full')
  expect(provider.recoveredItems).toEqual([item])
  await consumeRecovered(provider, item.id)
  expect(provider.recoveredItems).toEqual([])
  provider.destroy()
})
it('a failed close checkpoint keeps the dialog and work open until retry or cancellation', async () => {
  const keep = vi.fn().mockRejectedValueOnce(new Error('Disk full')).mockResolvedValue(undefined)
  const result = decideClose({
    dirty: true,
    pending: 2,
    keep,
    save: vi.fn(async () => true),
    discard: vi.fn(async () => {}),
  })
  const dialog = document.querySelector('dialog')!
  dialog.querySelector<HTMLButtonElement>('button.primary')!.click()
  await vi.waitFor(() =>
    expect(dialog.querySelector('[role=alert]')!.textContent).toContain('Disk full'),
  )
  expect(dialog.open).toBe(true)
  dialog.querySelector<HTMLButtonElement>('button.primary')!.click()
  await expect(result).resolves.toBe('keep')
  expect(document.querySelector('dialog')).toBeNull()
})
it('cancelling the save picker also cancels closure', async () => {
  const save = vi.fn(async () => false)
  const result = decideClose({
    dirty: true,
    pending: 0,
    keep: async () => {},
    save,
    discard: async () => {},
  })
  document.querySelector<HTMLButtonElement>('dialog button.primary')!.click()
  await expect(result).resolves.toBeNull()
  expect(save).toHaveBeenCalledOnce()
})

it('a contextual band accepts Ctrl/Cmd Enter through the common panel keyboard scope', () => {
  const form = contextPanel({
    id: 'note',
    closeLabel: 'Cancel',
    close: () => {},
    canConfirm: () => true,
  })
  const input = document.createElement('textarea')
  form.body.append(input)
  document.body.append(form.element)
  form.element.hidden = false
  const confirm = vi.fn()
  form.confirm = confirm
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true }))
  input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', metaKey: true, bubbles: true }))
  expect(confirm).toHaveBeenCalledTimes(2)
})

it('quitting from a clean view still asks to preserve work in the other windows', async () => {
  const keep = vi.fn(async () => {})
  const save = vi.fn(async () => true)
  const result = decideClose({
    dirty: false,
    pending: 0,
    otherWindows: 1,
    keep,
    save,
    discard: async () => {},
  })
  const dialog = document.querySelector('dialog')!
  expect(dialog.textContent).toContain('Leurs saisies en cours seront conservées.')
  dialog.querySelector<HTMLButtonElement>('button.primary')!.click()
  await expect(result).resolves.toBe('keep')
  expect(keep).toHaveBeenCalledOnce()
  expect(save).not.toHaveBeenCalled()
})

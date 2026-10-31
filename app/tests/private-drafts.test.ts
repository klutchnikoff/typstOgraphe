import { afterEach, expect, it, vi } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { PrivateDrafts, privateDrafts } from '../src/private-drafts'
import { trackForm } from '../src/form-drafts'
import { RecoveryScheduler } from '../src/recovery-scheduler'
import { formDialog } from '../src/form-dialog'

afterEach(() => {
  vi.useRealTimers()
  document.body.replaceChildren()
})

it('isolates owners, preserves detached forms and freezes only the composing form', () => {
  const changed = vi.fn(),
    drafts = new PrivateDrafts(changed)
  let text = 'Before',
    other = 'Other'
  const first = drafts.track('author', { collection: 'authors', id: 'a' }, () => ({ text }))
  drafts.track('affiliation', {}, () => ({ text: other }))
  first.beginComposition()
  text = 'Preedit に'
  other = 'Updated'
  expect(drafts.snapshot().map((d) => d.data)).toEqual([{ text: 'Before' }, { text: 'Updated' }])
  text = 'Validated にほん'
  first.endComposition()
  const snapshot = drafts.snapshot()
  first.detach()
  text = 'Destroyed source'
  expect(drafts.snapshot()[0]).toEqual(snapshot[0])
  const anotherWindow = new PrivateDrafts(() => {})
  expect(anotherWindow.size).toBe(0)
  first.discard()
  expect(drafts.snapshot().map((d) => d.data)).toEqual([{ text: 'Updated' }])
})

it('captures exact form values, choices, rich text and explicit resources without submitting', async () => {
  const owner = {},
    root = document.createElement('form')
  root.id = 'author-form'
  root.innerHTML =
    '<input id="name"><input id="flag" type="checkbox"><textarea id="raw"></textarea><div id="rich"></div>'
  document.body.append(root)
  const editor = new Editor({
    element: root.querySelector('#rich')!,
    extensions: [StarterKit],
    content: '<p>Abstract</p>',
  })
  const resource = { path: 'images/private.svg', data: 'bytes' }
  const source = trackForm(owner, root, { kind: 'record', collection: 'authors' }, () => ({
    resource,
  }))
  const input = root.querySelector<HTMLInputElement>('#name')!
  input.value = 'Camille é😀'
  input.dispatchEvent(new Event('input', { bubbles: true }))
  root.querySelector<HTMLInputElement>('#flag')!.checked = true
  root.querySelector<HTMLTextAreaElement>('#raw')!.value = 'Incomplete, unvalidated value'
  editor.commands.insertContent('Rich ')
  const snapshot = privateDrafts(owner).snapshot()[0]
  expect((snapshot.data as any).fields.map((f: any) => f.value)).toEqual([
    'Camille é😀',
    'on',
    'Incomplete, unvalidated value',
  ])
  expect((snapshot.data as any).fields[1].checked).toBe(true)
  expect(JSON.stringify((snapshot.data as any).rich)).toContain('Rich Abstract')
  expect((snapshot.data as any).data).toEqual({ resource })
  input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }))
  input.value = 'Uncommitted preedit'
  expect(JSON.stringify(privateDrafts(owner).snapshot())).not.toContain('Uncommitted')
  input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }))
  await Promise.resolve()
  expect(JSON.stringify(privateDrafts(owner).snapshot())).toContain('Uncommitted preedit')
  source.detach()
  editor.destroy()
  root.remove()
  expect(privateDrafts(owner).size).toBe(1)
  source.discard()
  expect(privateDrafts(owner).size).toBe(0)
})

it('writes after a pause and also at the maximum deadline under uninterrupted input', async () => {
  vi.useFakeTimers()
  const write = vi.fn(async () => {}),
    scheduler = new RecoveryScheduler(write, vi.fn())
  scheduler.changed()
  await vi.advanceTimersByTimeAsync(1999)
  expect(write).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(1)
  expect(write).toHaveBeenCalledTimes(1)
  scheduler.changed()
  for (let i = 0; i < 10; i++) {
    await vi.advanceTimersByTimeAsync(1000)
    scheduler.changed()
  }
  expect(write).toHaveBeenCalledTimes(2)
  scheduler.stop()
  await vi.advanceTimersByTimeAsync(20000)
  expect(write).toHaveBeenCalledTimes(2)
})

it('serializes writes, preserves edits made during IO and retries a failed write', async () => {
  vi.useFakeTimers()
  let finish!: () => void
  const write = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve
        }),
    )
    .mockResolvedValue(undefined)
  const failed = vi.fn(),
    scheduler = new RecoveryScheduler(write, failed)
  scheduler.changed()
  const first = scheduler.flush()
  scheduler.changed()
  expect(write).toHaveBeenCalledTimes(1)
  finish()
  await first
  expect(write).toHaveBeenCalledTimes(2)
  write.mockRejectedValueOnce(new Error('Disk full'))
  scheduler.changed()
  await vi.advanceTimersByTimeAsync(2000)
  expect(failed).toHaveBeenCalledTimes(1)
  await vi.advanceTimersByTimeAsync(2000)
  expect(write).toHaveBeenCalledTimes(4)
  scheduler.stop()
})

it('detaches a closed search draft before the sidebar is reused for global preferences', () => {
  const owner = {},
    sidebar = document.createElement('aside'),
    search = document.createElement('section')
  search.id = 'document-search-draft'
  search.innerHTML = '<input id="replace" value="Private replacement">'
  sidebar.append(search)
  document.body.append(sidebar)
  const tracked = trackForm(owner, search, { kind: 'search-replace' })
  const before = privateDrafts(owner).snapshot()
  search.remove()
  sidebar.innerHTML = '<input id="personal-email" value="global@example.org">'
  expect(privateDrafts(owner).snapshot()).toEqual(before)
  search.querySelector('input')!.value = 'Destroyed source'
  expect(privateDrafts(owner).snapshot()).toEqual(before)
  expect(JSON.stringify(before)).not.toContain('global@example.org')
  tracked.discard()
})

it('detached dialogs keep their typed draft getter in the common collector', () => {
  const owner = {},
    form = formDialog({ id: 'record', title: 'Author', cancel() {}, returnFocus: () => undefined })
  let name = 'Initial'
  form.recoveryData = () => ({ record: { name, affiliations: ['official-affiliation'] } })
  form.trackRecovery(owner, { kind: 'record', collection: 'authors' })
  name = 'Typed private author'
  const snapshot = privateDrafts(owner).snapshot()[0]
  expect((snapshot.data as any).data.record).toEqual({
    name: 'Typed private author',
    affiliations: ['official-affiliation'],
  })
  form.destroy()
  name = 'Destroyed source'
  expect(privateDrafts(owner).snapshot()[0]).toEqual(snapshot)
})

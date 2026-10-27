import { afterEach, expect, it, vi } from 'vitest'
import { Editor, type JSONContent } from '@tiptap/core'
import { richZoneExtensions } from '../src/rich-zone'
import { proposalDetails, proposalPreview } from '../src/proposal-preview'
import { preferencesPanel, type Preferences } from '../src/preferences-panel'
import { recoveryPanel } from '../src/recovery-panel'
import { proofingKey, searchState, navigateSpelling } from '../src/proofing'
import { navigateSearch } from '../src/search-panel'
import type { Catalogue } from '../src/shell'
import type { Transport } from '../src/provider'

const editors: Editor[] = []
afterEach(() => {
  editors.forEach((editor) => editor.destroy())
  editors.length = 0
  document.body.replaceChildren()
})
const paragraph = (text: string): JSONContent => ({
  type: 'doc',
  content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
})
it('rend les styles, notes, liens et ressources dans la relecture, sans interpréter le texte', () => {
  const before = paragraph('<script>test</script>')
  const after = structuredClone(before)
  after.content![0].content![0].marks = [
    { type: 'bold' },
    { type: 'link', attrs: { href: 'https://example.org' } },
  ]
  after.content![0].content!.push({ type: 'footnote', attrs: { text: 'Précision' } })
  after.content!.push({
    type: 'figure',
    attrs: { src: 'images/test.png', alt: 'Image décrite' },
  })
  const imageURL = vi.fn(() => 'blob:test')
  const preview = proposalPreview(after, imageURL)
  expect(preview.querySelector('strong')?.textContent).toBe('<script>test</script>')
  expect(preview.querySelector('script')).toBeNull()
  expect(preview.querySelector('a')?.getAttribute('href')).toBe('https://example.org')
  expect(preview.textContent).toContain('Précision')
  expect(preview.querySelector('img')?.getAttribute('src')).toBe('blob:test')
  expect(imageURL).toHaveBeenCalledWith('images/test.png')
  expect(proposalDetails(before, after)).toEqual(
    expect.arrayContaining([
      'Mise en forme modifiée',
      'Structure modifiée',
      'Lien :  → https://example.org',
    ]),
  )
  const changedNote = structuredClone(after)
  changedNote.content![0].content![1].attrs!.text = 'Note corrigée'
  expect(proposalDetails(after, changedNote)).toContain(
    'Note de bas de page : Précision → Note corrigée',
  )
})
it('navigue en boucle sans cumuler les sélections, en gardant les autres occurrences marquées', () => {
  const editor = new Editor({
    extensions: richZoneExtensions('document'),
    content: paragraph('été été été'),
  })
  editors.push(editor)
  expect(searchState(editor, 'ÉTÉ')).toHaveLength(3)
  editor.commands.setTextSelection(1)
  navigateSearch(editor, false, false)
  expect(editor.state.selection.from).toBe(1)
  navigateSearch(editor, false, false)
  expect(editor.state.selection.from).toBe(5)
  navigateSearch(editor, true, false)
  expect(editor.state.selection.from).toBe(1)
  navigateSearch(editor, true, false)
  expect(editor.state.selection.from).toBe(9)
  expect(proofingKey.getState(editor.state)?.hits).toHaveLength(3)
  editor.view.dispatch(
    editor.state.tr.setMeta(proofingKey, {
      issues: [
        { from: 1, to: 4, word: 'été' },
        { from: 9, to: 12, word: 'été' },
      ],
    }),
  )
  navigateSpelling(editor)
  expect(editor.state.selection.from).toBe(1)
  navigateSpelling(editor)
  expect(editor.state.selection.from).toBe(9)
})
it('ne modifie le dictionnaire et la signature des préférences qu’à la validation', async () => {
  const preferences: Preferences = {
    appearance: 'system',
    font: 'serif',
    size: 18,
    personal: { fr: ['Typst'] },
    profile: {},
    signature: { path: 'images/sign.png', data: 'AA==' },
    recent: [],
    editor_fraction: 0.6,
    spelling_enabled: true,
  }
  const saved = vi.fn(async (_: Preferences) => {})
  const root = document.createElement('div')
  document.body.append(root)
  preferencesPanel(
    root,
    { types: [{ fields: [] }] } as unknown as Catalogue,
    preferences,
    {} as Transport,
    saved,
    (e) => {
      throw e
    },
  )
  expect(root.querySelector('img')?.src).toContain('data:image/png;base64,')
  root.querySelector<HTMLButtonElement>('[aria-label="Supprimer Typst"]')!.click()
  ;[...root.querySelectorAll('button')]
    .find((b) => b.textContent === 'Supprimer la signature')!
    .click()
  root.querySelector<HTMLInputElement>('[aria-label="Correction orthographique"]')!.checked = false
  expect(preferences.personal.fr).toEqual(['Typst'])
  expect(preferences.signature).not.toBeNull()
  root.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }))
  await Promise.resolve()
  const next = saved.mock.calls[0][0]
  expect(next.signature).toBeNull()
  expect(next.personal.fr).toEqual([])
  expect(next.spelling_enabled).toBe(false)
  expect(next.editor_fraction).toBe(0.6)
})
it('liste les copies de récupération, distingue les copies invalides et supprime explicitement', async () => {
  let entries = [
    { id: 'one', name: 'Lettre.typsto', savedAt: 123, error: null as string | null },
    { id: 'two', name: 'Copie illisible', savedAt: 0, error: 'invalide' },
  ]
  const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (command === 'recovery_list') return entries
    if (command === 'recovery_delete') entries = entries.filter((e) => e.id !== args!.id)
  })
  const root = document.createElement('div'),
    restore = vi.fn(),
    failed = vi.fn()
  await recoveryPanel(root, { invoke } as unknown as Transport, restore, failed)
  const recover = [...root.querySelectorAll('button')].filter((b) => b.textContent === 'Récupérer')
  expect(recover[1].disabled).toBe(true)
  recover[0].click()
  expect(restore).toHaveBeenCalledWith('one')
  ;[...root.querySelectorAll('button')].find((b) => b.textContent === 'Supprimer la copie')!.click()
  await vi.waitFor(() => expect(root.textContent).not.toContain('Lettre.typsto'))
  expect(failed).not.toHaveBeenCalled()
})

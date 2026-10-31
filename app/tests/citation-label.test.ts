import { Editor } from '@tiptap/core'
import { afterEach, expect, it } from 'vitest'
import { setCitationEntries, type Citation, type ReferenceEntry } from '../src/citation-index'
import { richZoneExtensions } from '../src/rich-zone'

const editors: Editor[] = []
afterEach(() => editors.splice(0).forEach((editor) => editor.destroy()))
const entry: ReferenceEntry = {
  file_id: 'books',
  file_name: 'books.bib',
  key: 'imaginaire1200',
  authors: 'Imaginaire',
  title: 'Théorème de représentation',
  year: '1200',
  label: 'Imaginaire, 1200',
}
function create(form: Citation['form'], multiple = false) {
  const editor = new Editor({
    extensions: richZoneExtensions('document'),
    content: {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'citation',
              attrs: {
                form,
                items: [
                  { file_id: entry.file_id, key: entry.key, supplement: '' },
                  ...(multiple ? [{ file_id: 'books', key: 'unresolved', supplement: '' }] : []),
                ],
              },
            },
          ],
        },
      ],
    },
  })
  editors.push(editor)
  setCitationEntries(editor, [entry])
  return editor
}

it.each([
  ['normal', false, '(Imaginaire, 1200)'],
  ['normal', true, '(Imaginaire, 1200 ; unresolved)'],
  ['prose', false, 'Imaginaire, 1200'],
  ['prose', true, 'Imaginaire, 1200 ; unresolved'],
] as const)(
  'présente la citation %s (groupe : %s) sans modifier son contenu',
  (form, multiple, label) => {
    const editor = create(form, multiple)
    expect(editor.view.dom.querySelector('.inline-object-label')?.textContent).toBe(label)
    expect(editor.state.doc.firstChild!.firstChild!.attrs.form).toBe(form)
    expect(editor.getText()).not.toContain('(')
    expect(editor.getHTML()).not.toContain(label)
  },
)

it('actualise les parenthèses à un changement de forme et le libellé à un nouvel index', () => {
  const editor = create('normal')
  const badge = editor.view.dom.querySelector('.inline-object-label')!
  editor.view.dispatch(editor.state.tr.setNodeAttribute(1, 'form', 'prose'))
  expect(editor.view.dom.querySelector('.inline-object-label')).toBe(badge)
  expect(badge.textContent).toBe('Imaginaire, 1200')
  editor.view.dispatch(editor.state.tr.setNodeAttribute(1, 'form', 'normal'))
  const doc = editor.getJSON()
  setCitationEntries(editor, [{ ...entry, year: '1201', label: 'Imaginaire, 1201' }])
  expect(badge.textContent).toBe('(Imaginaire, 1201)')
  expect(editor.getJSON()).toEqual(doc)
})

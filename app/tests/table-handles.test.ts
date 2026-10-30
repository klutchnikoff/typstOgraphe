import { Editor } from '@tiptap/core'
import { afterEach, expect, it, vi } from 'vitest'
import { richZoneExtensions } from '../src/rich-zone'
import { bindTableHandles, refreshTableHandles, selectTableAxis } from '../src/table-handles'
const editors: Editor[] = []
afterEach(() => {
  editors.splice(0).forEach((e) => e.destroy())
  document.body.replaceChildren()
})
function create() {
  const element = document.createElement('div')
  document.body.append(element)
  const cell = (type: string, text: string) => ({
    type,
    content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
  })
  const e = new Editor({
    element,
    extensions: richZoneExtensions('document'),
    content: {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Avant' }] },
        {
          type: 'tableFigure',
          content: [
            {
              type: 'table',
              content: [
                { type: 'tableRow', content: [cell('tableHeader', 'A'), cell('tableHeader', 'B')] },
                { type: 'tableRow', content: [cell('tableCell', 'C'), cell('tableCell', 'D')] },
              ],
            },
            { type: 'tableCaption', content: [{ type: 'text', text: 'Légende' }] },
          ],
        },
      ],
    },
  })
  editors.push(e)
  e.commands.setTextSelection(1)
  return e
}
it('les contrôles ne changent ni le JSON ni le HTML exporté, et visent la cellule sans déplacer le curseur', () => {
  const e = create(),
    before = e.getJSON(),
    html = e.getHTML(),
    open = vi.fn()
  bindTableHandles(e, { available: () => true, open })
  expect(e.view.dom.querySelectorAll('.table-axis-handle')).toHaveLength(4)
  const button = e.view.dom.querySelector<HTMLButtonElement>(
    '[data-table-axis="column"][data-table-index="1"]',
  )!
  button.click()
  expect(open.mock.calls[0][0]).toMatchObject({ pos: 7, row: 0, column: 1 })
  expect(open.mock.calls[0][1]).toBe('column')
  expect(e.state.selection.from).toBe(1)
  expect(e.getJSON()).toEqual(before)
  expect(e.getHTML()).toBe(html)
  expect(e.view.dom.querySelectorAll('.table-handle')).toHaveLength(4)
})
it('la disponibilité est commune, et les poignées suivent leur tableau après une insertion dans le texte précédent', () => {
  const e = create(),
    open = vi.fn()
  let enabled = true
  bindTableHandles(e, { available: () => enabled, open })
  e.commands.insertContentAt(1, '日本😀')
  e.view.dom
    .querySelector<HTMLButtonElement>('[data-table-axis="row"][data-table-index="1"]')!
    .click()
  expect(open.mock.calls[0][0]).toMatchObject({ pos: 11, row: 1, column: 0 })
  enabled = false
  refreshTableHandles(e)
  const buttons = [...e.view.dom.querySelectorAll<HTMLButtonElement>('.table-handle')]
  expect(buttons.every((b) => b.disabled)).toBe(true)
  buttons[0].click()
  expect(open).toHaveBeenCalledTimes(1)
})
it('le repérage reste local, suit la position du widget et disparaît si le tableau change', () => {
  const e = create(),
    node = e.state.doc.nodeAt(7)!,
    before = e.getJSON()
  selectTableAxis(e, { pos: 7, node, axis: 'column', index: 1 })
  expect(e.view.dom.querySelectorAll('.table-axis-selected')).toHaveLength(2)
  expect(e.getJSON()).toEqual(before)
  e.commands.insertContentAt(1, 'X')
  expect(e.view.dom.querySelectorAll('.table-axis-selected')).toHaveLength(2)
  let point = 0
  e.state.doc.descendants((n, pos) => {
    if (n.type.name === 'tableHeader' && !point) point = pos + 2
  })
  e.commands.insertContentAt(point, 'Changed')
  expect(e.view.dom.querySelectorAll('.table-axis-selected')).toHaveLength(0)
})

it('les portées des en-têtes survivent au HTML et une portée inconnue reste un en-tête ordinaire', () => {
  const e = create()
  e.commands.setContent(
    '<figure data-table><table><tbody><tr><th scope="col" data-header-scope="both"><p>Angle</p></th><th scope="col"><p>Colonne</p></th></tr><tr><th scope="row"><p>Ligne</p></th><td><p>Donnée</p></td></tr></tbody></table><figcaption data-table-caption>Légende</figcaption></figure>',
  )
  expect(e.view.dom.querySelector('[data-header-scope="both"]')?.textContent).toBe('Angle')
  expect(e.view.dom.querySelector('th[scope="row"]')?.textContent).toBe('Ligne')
  const before = e.getJSON()
  e.commands.setContent(e.getHTML())
  expect(e.getJSON()).toEqual(before)
  e.commands.setContent(
    '<table><tbody><tr><th data-header-scope="unknown"><p>Nom</p></th></tr></tbody></table>',
  )
  expect(e.view.dom.querySelector('th')?.getAttribute('data-header-scope')).toBe('column')
})

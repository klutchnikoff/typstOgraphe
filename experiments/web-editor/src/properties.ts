import { getMarkRange, type Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import type { BandHost } from './band-layout'
import { convertList, listKinds, type ListKind } from './editing'
import { tr } from './i18n'
import type { Catalogue } from './shell'

export function propertyPanel(
  editor: Editor,
  bands: BandHost,
  catalogue: Catalogue,
  chooseImage: () => void,
) {
  const form = document.createElement('form')
  form.id = 'properties'
  form.tabIndex = -1
  form.noValidate = true
  form.className = 'properties'
  form.hidden = true
  form.innerHTML = `<div class="panel-title"><strong id="property-title"></strong><button type="button" id="close-properties">×</button></div>
    <div id="list-options" class="segmented" role="radiogroup"></div>
    <div id="start-field"><label for="list-start"></label><input id="list-start" type="number" min="1" max="10000"></div>
    <label id="property-label" for="property-value"></label><input id="property-inline" type="text"><textarea id="property-value" rows="2"></textarea>
    <div id="image-options"><button type="button" id="replace-image"></button><span id="image-help"></span></div>
    <p id="property-help" class="band-hint"></p>
    <div class="panel-actions"><button type="button" id="remove-block" class="danger"></button><button type="button" id="cancel-properties"></button><button type="submit" id="apply-properties" class="primary"></button></div>
    <p id="property-error" role="alert"></p>`
  const $ = <T extends HTMLElement = HTMLElement>(id: string) => form.querySelector<T>(`#${id}`)!
  const value = $<HTMLTextAreaElement>('property-value')
  const inlineValue = $<HTMLInputElement>('property-inline')
  const field = () => (opened?.kind === 'footnote' ? value : inlineValue)
  const initial = $<HTMLInputElement>('list-start')
  let opened:
    | {
        kind: string
        node?: PMNode
        original: string
        list: ListKind
        remove: boolean
        replacement?: string
      }
    | undefined
  const names = [tr('Liste à puces'), tr('Liste numérotée'), tr('Descriptions')]
  $('list-options').setAttribute('aria-label', tr('Type de liste'))
  listKinds.forEach((kind, index) => {
    const button = document.createElement('button')
    button.type = 'button'
    button.role = 'radio'
    button.setAttribute('aria-label', names[index])
    button.textContent = names[index]
    const select = () => {
      if (!opened) return
      opened.list = kind
      renderList()
    }
    button.onclick = select
    button.onkeydown = (event) => {
      if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return
      event.preventDefault()
      const next = (index + (event.key === 'ArrowLeft' ? 2 : 1)) % 3
      const nextButton = $('list-options').children[next] as HTMLButtonElement
      nextButton.click()
      nextButton.focus()
    }
    $('list-options').append(button)
  })
  function renderList() {
    for (const [index, button] of [...$('list-options').children].entries()) {
      button.setAttribute('aria-checked', String(listKinds[index] === opened?.list))
      ;(button as HTMLButtonElement).tabIndex = listKinds[index] === opened?.list ? 0 : -1
    }
    $('start-field').hidden = opened?.list !== 'orderedList'
  }
  const close = () => bands.close()
  function valid() {
    const anchor = bands.target,
      current = opened
    if (!anchor || !current) return false
    const node = editor.state.doc.nodeAt(anchor.from)
    const stale =
      current.kind === 'link'
        ? editor.state.doc.textBetween(anchor.from, anchor.to) !== current.original
        : !node?.eq(current.node!)
    if (stale)
      $('property-error').textContent = tr(
        'Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.',
      )
    return !stale
  }
  function remove() {
    if (!valid()) return
    const { from, to } = bands.target!
    if (opened!.kind === 'link') editor.chain().setTextSelection({ from, to }).unsetLink().run()
    else editor.commands.deleteRange({ from, to })
    close()
  }
  form.onsubmit = (event) => {
    event.preventDefault()
    if (editor.view.composing || !opened || !valid()) return
    if (opened.remove) {
      remove()
      return
    }
    const anchor = bands.target!,
      node = editor.state.doc.nodeAt(anchor.from)!
    if (opened.kind === 'link') {
      const href = field().value.trim()
      if (!/^(https?:\/\/|mailto:)/.test(href)) {
        $('property-error').textContent = tr('Utilisez une adresse http, https ou mailto.')
        return
      }
      editor.chain().setTextSelection({ from: anchor.from, to: anchor.to }).setLink({ href }).run()
    } else if (listKinds.includes(opened.kind as ListKind)) {
      const start = Number(initial.value)
      if (
        opened.list === 'orderedList' &&
        (!Number.isInteger(start) || start < 1 || start > 10000)
      ) {
        $('property-error').textContent = tr('Numéro initial entre 1 et 10 000.')
        initial.focus()
        return
      }
      const next = opened.list === opened.kind ? node.toJSON() : convertList(node, opened.list)
      if (opened.list === 'orderedList') next.attrs = { start }
      editor.commands.insertContentAt({ from: anchor.from, to: anchor.to }, next)
    } else if (['figure', 'footnote'].includes(opened.kind)) {
      const attrs =
        opened.kind === 'figure'
          ? { ...node.attrs, alt: field().value, src: opened.replacement || node.attrs.src }
          : { ...node.attrs, text: field().value }
      editor.view.dispatch(editor.state.tr.setNodeMarkup(anchor.from, undefined, attrs))
    }
    close()
  }
  $('close-properties').setAttribute('aria-label', tr('Fermer les paramètres'))
  $('close-properties').onclick = $('cancel-properties').onclick = close
  $('cancel-properties').textContent = tr('Annuler')
  $('remove-block').textContent = tr('Supprimer')
  $('remove-block').onclick = remove
  $('replace-image').textContent = tr('Remplacer…')
  $('replace-image').onclick = chooseImage
  $('image-help').textContent = tr('La légende se modifie directement dans le bloc.')
  $('start-field').querySelector('label')!.textContent = tr('Numéro initial')
  return {
    open(pos: number, kind: string, remove = false) {
      if (editor.view.composing) return
      const node = editor.state.doc.nodeAt(pos)
      if (!node || (kind !== 'link' && node.type.name !== kind)) return
      const selection = editor.state.selection
      const range =
        kind === 'link'
          ? getMarkRange(editor.state.doc.resolve(pos), editor.schema.marks.link) || selection
          : { from: pos, to: pos + node.nodeSize }
      const original = kind === 'link' ? editor.state.doc.textBetween(range.from, range.to) : ''
      const current = { kind, node, original, list: kind as ListKind, remove }
      // Close the old interaction before publishing this one's local draft.
      bands.close(false)
      opened = current
      const list = listKinds.includes(kind as ListKind)
      const editable = !remove && ['link', 'figure', 'footnote'].includes(kind)
      $('property-title').textContent = remove
        ? tr('Supprimer le bloc')
        : list
          ? tr('Paramètres de la liste')
          : kind === 'link'
            ? tr('Lien')
            : kind === 'figure'
              ? tr('Paramètres de l’image')
              : kind === 'footnote'
                ? tr('Note de bas de page')
                : catalogue.entries.find((c) => c.node === kind)?.label || tr('Paramètres du bloc')
      $('list-options').hidden = !list || remove
      $('start-field').hidden = true
      if (list) renderList()
      if (remove) $('start-field').hidden = true
      initial.value = String(node.attrs.start || 1)
      $('property-label').textContent =
        kind === 'link'
          ? tr('Adresse du lien')
          : kind === 'figure'
            ? tr('Texte alternatif')
            : tr('Texte de la note')
      $('property-label').hidden = !editable
      $('property-label').setAttribute('for', field().id)
      value.hidden = !editable || kind !== 'footnote'
      inlineValue.hidden = !editable || kind === 'footnote'
      field().value =
        kind === 'link'
          ? editor.getAttributes('link').href || ''
          : kind === 'figure'
            ? node.attrs.alt || ''
            : node.attrs.text || ''
      $('replace-image').textContent = tr('Remplacer…')
      $('image-options').hidden = kind !== 'figure' || remove
      $('property-help').textContent = remove
        ? tr('Le bloc entier sera supprimé. Vous pourrez annuler cette suppression.')
        : list
          ? tr('Ces paramètres concernent la liste entière.')
          : editable
            ? ''
            : tr('La présentation est définie par le modèle.')
      $('property-error').textContent = ''
      $('remove-block').hidden = remove
      $('apply-properties').textContent = remove ? tr('Supprimer') : tr('Appliquer')
      $('apply-properties').classList.toggle('danger', remove)
      bands.open(form, range.from, range.to, !['link', 'footnote'].includes(kind), () => {
        opened = undefined
      })
      opened = current
      if (remove) $('apply-properties').focus()
      else if (list) $('list-options').querySelector<HTMLElement>('[aria-checked=true]')?.focus()
      else if (editable) field().focus()
      else $('apply-properties').focus()
      bands.reveal()
    },
    replaceImage(path: string) {
      if (opened?.kind !== 'figure' || opened.remove || !valid()) return false
      opened.replacement = path
      $('replace-image').textContent = tr('Image remplacée — appliquer pour confirmer')
      $('replace-image').focus()
      return true
    },
  }
}

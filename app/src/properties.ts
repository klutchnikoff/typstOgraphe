import { navigationGroup } from './keyboard-navigation'
import { getMarkRange, type Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import type { BandHost } from './band-layout'
import { convertList, listKinds, type ListKind } from './editing'
import { tr } from './i18n'
import type { Catalogue } from './shell'
import { contextPanel } from './context-panel'
import { removeWidget } from './widgets'
import { isWidgetKind, widgetLabel } from './widget-frame'
import { hasSettings, settingsForNode, type Scalar } from './capability-settings'
import { ParameterFields } from './parameter-fields'

export function propertyPanel(
  editor: Editor,
  bands: BandHost,
  catalogue: Catalogue,
  chooseImage: () => void,
  idPrefix = '',
) {
  const panel = contextPanel({
    id: `${idPrefix}properties`,
    titleId: `${idPrefix}property-title`,
    closeId: `${idPrefix}close-properties`,
    errorId: `${idPrefix}property-error`,
    closeLabel: tr('Fermer les paramètres'),
    close: () => bands.close(),
    canConfirm: () => !editor.view.composing,
    tabExit: () => editor.view.focus(),
  })
  const form = panel.element
  const group = document.createElement('div')
  group.id = `${idPrefix}list-options`
  group.className = 'segmented'
  group.role = 'radiogroup'
  group.setAttribute('aria-label', tr('Type de liste'))
  navigationGroup(group, { axis: 'horizontal', roving: true, activateOnMove: true })
  const fieldsRoot = document.createElement('div')
  const hint = document.createElement('p')
  hint.className = 'band-hint'
  panel.body.append(group, fieldsRoot, hint)
  const fields = new ParameterFields(fieldsRoot, chooseImage, idPrefix)
  const removeAction = panel.action({
    id: `${idPrefix}remove-block`,
    label: tr('Supprimer'),
    danger: true,
    run: () => {
      if (!valid()) return
      const target = bands.target
      if (opened && target && opened.kind !== 'link') api.open(target.from, opened.kind, true)
      else remove()
    },
  })
  panel.action({
    id: `${idPrefix}cancel-properties`,
    label: tr('Annuler'),
    run: () => bands.close(),
  })
  const apply = panel.action({
    id: `${idPrefix}apply-properties`,
    label: tr('Valider'),
    primary: true,
    run: () => panel.submit(),
  })
  let opened:
    | {
        kind: string
        selected: string
        node: PMNode
        original: string
        remove: boolean
        drafts: Record<string, Scalar | undefined>
      }
    | undefined
  const close = () => bands.close()
  function valid() {
    const anchor = bands.target,
      current = opened
    if (!anchor || !current) return false
    const node = editor.state.doc.nodeAt(anchor.from)
    const stale =
      current.kind === 'link'
        ? editor.state.doc.textBetween(anchor.from, anchor.to) !== current.original
        : !node?.eq(current.node)
    if (stale)
      panel.error.textContent = tr(
        'Ce bloc a été modifié ailleurs. Fermez puis rouvrez ses paramètres.',
      )
    return !stale
  }
  function remove() {
    if (!valid()) return
    const { from, to } = bands.target!
    if (opened!.kind === 'link') editor.chain().setTextSelection({ from, to }).unsetLink().run()
    else if (isWidgetKind(opened!.kind))
      removeWidget(editor, { pos: from, node: editor.state.doc.nodeAt(from)! })
    else editor.commands.deleteRange({ from, to })
    close()
  }
  function render() {
    const current = opened!
    const contract = settingsForNode(current.selected, catalogue.entries)
    group.replaceChildren()
    group.hidden = current.remove || contract.conversions.length < 2
    for (const [index, capability] of contract.conversions.entries()) {
      const kind = capability.node!
      const button = document.createElement('button')
      button.type = 'button'
      button.role = 'radio'
      button.textContent = widgetLabel(kind)
      button.setAttribute('aria-label', button.textContent)
      button.setAttribute('aria-checked', String(current.selected === kind))
      button.tabIndex = current.selected === kind ? 0 : -1
      button.onclick = () => {
        current.selected = kind
        render()
        group.querySelector<HTMLElement>('[aria-checked=true]')?.focus()
      }
      group.append(button)
    }
    fields.render(contract.fields, current.drafts)
    hint.textContent =
      contract.conversions.length > 1 ? tr('Ces paramètres concernent la liste entière.') : ''
  }
  panel.confirm = () => {
    if (editor.view.composing || !opened || !valid()) return
    if (opened.remove) {
      remove()
      return
    }
    const result = fields.read()
    if ('error' in result) {
      panel.error.textContent = result.error!
      return
    }
    const anchor = bands.target!,
      node = editor.state.doc.nodeAt(anchor.from)!
    if (opened.kind === 'link') {
      editor
        .chain()
        .setTextSelection({ from: anchor.from, to: anchor.to })
        .setLink({ href: String(result.attrs.href) })
        .run()
    } else {
      const next =
        opened.selected !== opened.kind && listKinds.includes(opened.selected as ListKind)
          ? convertList(node, opened.selected as ListKind)
          : node.toJSON()
      next.attrs = { ...next.attrs, ...result.attrs }
      const replacement = editor.schema.nodeFromJSON(next)
      if (!node.eq(replacement))
        editor.view.dispatch(editor.state.tr.replaceWith(anchor.from, anchor.to, replacement))
    }
    close()
  }
  const api = {
    open(pos: number, kind: string, remove = false) {
      if (editor.view.composing) return
      const contract = settingsForNode(kind, catalogue.entries)
      if (!remove && !hasSettings(contract)) return
      const node = editor.state.doc.nodeAt(pos)
      if (!node || (kind !== 'link' && node.type.name !== kind)) return
      const selection = editor.state.selection
      const range =
        kind === 'link'
          ? getMarkRange(editor.state.doc.resolve(pos), editor.schema.marks.link) || selection
          : { from: pos, to: pos + node.nodeSize }
      const attrs = kind === 'link' ? editor.getAttributes('link') : node.attrs
      const drafts: Record<string, Scalar | undefined> = {}
      for (const field of contract.fields)
        drafts[field.name] = attrs[field.attribute] ?? field.schema.default
      bands.close(false)
      opened = {
        kind,
        selected: kind,
        node,
        original: kind === 'link' ? editor.state.doc.textBetween(range.from, range.to) : '',
        remove,
        drafts,
      }
      const current = opened
      render()
      panel.title.textContent = remove
        ? kind === 'footnote'
          ? tr('Supprimer la note de bas de page')
          : tr('Supprimer le bloc')
        : contract.conversions.length > 1
          ? tr('Paramètres de la liste')
          : kind === 'link'
            ? tr('Lien')
            : kind === 'figure'
              ? tr('Paramètres de l’image')
              : kind === 'footnote'
                ? tr('Note de bas de page')
                : widgetLabel(kind)
      panel.body.hidden = remove
      form.dataset.intent = remove ? 'remove' : 'settings'
      panel.error.textContent = ''
      removeAction.hidden = remove || isWidgetKind(kind)
      removeAction.textContent = kind === 'link' ? tr('Retirer le lien') : tr('Supprimer')
      apply.textContent = remove ? tr('Supprimer') : tr('Valider')
      apply.classList.toggle('danger', remove)
      apply.toggleAttribute('data-initial-focus', remove)
      bands.open(
        panel,
        {
          from: range.from,
          to: range.to,
          placement: isWidgetKind(kind) ? 'widget-header' : 'text',
        },
        () => {
          opened = undefined
        },
      )
      opened = current
    },
    replaceImage(path: string) {
      return !!opened && !opened.remove && valid() && fields.replaceImage(path)
    },
  }
  return api
}

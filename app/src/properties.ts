import { focusEditorSelection } from './editor-focus'
import { editingAvailability } from './command-availability'
import { navigationGroup } from './keyboard-navigation'
import { getMarkRange, type Editor } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { NodeSelection, Selection, TextSelection } from '@tiptap/pm/state'
import type { BandHost } from './band-layout'
import { convertList, listKinds, type ListKind } from './editing'
import { resourceCommand, type ImageResource } from './resource-commands'
import { tr } from './i18n'
import type { Catalogue } from './shell'
import { contextPanel } from './context-panel'
import { isWidgetKind, widgetLabel } from './widget-frame'
import { hasSettings, settingsForNode, type Scalar } from './capability-settings'
import { ParameterFields } from './parameter-fields'
import type { StructuredCommand } from './structured-commands'
import { applyFormatting } from './formatting'
import { applyObjectEdit, removeObject, treeTarget } from './object-commands'
import { editingProvider } from './editing-history'
import { documentContent } from './shared-inline-marks'

export function propertyPanel(
  editor: Editor,
  bands: BandHost,
  catalogue: Catalogue,
  chooseImage: () => void,
  idPrefix = '',
) {
  let pending = false
  const panel = contextPanel({
    id: `${idPrefix}properties`,
    titleId: `${idPrefix}property-title`,
    closeId: `${idPrefix}close-properties`,
    errorId: `${idPrefix}property-error`,
    closeLabel: tr('Fermer les paramètres'),
    close: () => {
      if (!pending) bands.close()
    },
    canConfirm: () => !pending && !editor.view.composing,
    tabExit: () => focusEditorSelection(editor),
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
      else void remove()
    },
  })
  panel.action({
    id: `${idPrefix}cancel-properties`,
    label: tr('Annuler'),
    run: () => {
      if (!pending) bands.close()
    },
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
        resource?: ImageResource
        drafts: Record<string, Scalar | undefined>
      }
    | undefined
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
  async function remove() {
    if (pending) return
    if (!valid()) return
    const { from, to } = bands.target!
    if (opened!.kind === 'link') {
      await changeLink('text.remove_link', { from, to })
      return
    }
    await commit((current) => removeObject(editor, from, false, current))
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
  async function changeLink(
    id: 'text.link' | 'text.remove_link',
    range: { from: number; to: number },
    url?: string,
  ) {
    await commit((current) => applyFormatting(editor, id, { range, url, focus: false, current }))
  }
  async function commit(change: (current: () => boolean) => Promise<boolean>) {
    const state = opened
    const current = () => opened === state && bands.forPanel(form) !== null
    pending = true
    apply.disabled = true
    removeAction.disabled = true
    try {
      await change(current)
      if (current()) bands.close(true, editor.state.selection.getBookmark())
      else if (!bands.isOpen && document.activeElement === document.body)
        focusEditorSelection(editor)
    } catch (error) {
      if (current())
        panel.error.textContent = error instanceof Error ? error.message : String(error)
    } finally {
      pending = false
      apply.disabled = false
      removeAction.disabled = false
    }
  }
  panel.confirm = () => {
    if (pending || editor.view.composing || !opened || !valid()) return
    if (opened.remove) {
      void remove()
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
      void changeLink('text.link', { from: anchor.from, to: anchor.to }, String(result.attrs.href))
      return
    } else {
      const next =
        opened.selected !== opened.kind && listKinds.includes(opened.selected as ListKind)
          ? convertList(node, opened.selected as ListKind)
          : node.toJSON()
      next.attrs = { ...next.attrs, ...result.attrs }
      const replacement = editor.schema.nodeFromJSON(next)
      const conversion = node.type !== replacement.type
      const transaction = conversion
        ? editor.state.tr.replaceWith(anchor.from, anchor.to, replacement)
        : editor.state.tr.setNodeMarkup(anchor.from, undefined, replacement.attrs)
      // Attribute changes preserve all positions. ProseMirror implements
      // setNodeMarkup as replacement for atoms and may otherwise drop a node
      // selection before the badge, or move a caret out of its caption.
      if (!conversion) {
        const selection = editor.state.selection
        transaction.setSelection(
          selection instanceof NodeSelection && selection.node.isInline
            ? TextSelection.create(transaction.doc, selection.to)
            : Selection.fromJSON(transaction.doc, selection.toJSON()),
        )
      }
      const commands: StructuredCommand[] | undefined = editingProvider(editor)
        ? [
            {
              id: conversion ? 'block.replace' : 'node.set_attrs',
              params: {
                target: treeTarget(editor, anchor.from),
                ...(conversion
                  ? { node: documentContent(replacement.toJSON()) }
                  : { attrs: result.attrs }),
              },
            },
          ]
        : undefined
      if (commands && opened.resource) commands.unshift(resourceCommand(opened.resource))
      void commit((current) =>
        applyObjectEdit(editor, transaction, commands, { focus: false, current }),
      )
    }
  }
  const api = {
    open(pos: number, kind: string, remove = false) {
      if (pending || !editingAvailability(editor).enabled) return
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
        ? kind === 'citation'
          ? tr('Supprimer la citation')
          : kind === 'footnote'
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
    imageReplacement() {
      if (!opened || opened.kind !== 'figure' || opened.remove) return undefined
      const current = opened
      return (resource: ImageResource) => {
        if (opened !== current || !valid() || !fields.replaceImage(resource.path))
          throw new Error(tr('Le document a changé. Réessayez.'))
        opened.resource = resource
      }
    },
  }
  return api
}

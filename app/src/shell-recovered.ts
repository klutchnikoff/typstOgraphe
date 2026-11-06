import { resumeBibliography } from './bibliography-manager'
import { reopenBand } from './form-recovery'
import { tr } from './i18n'
import { metadataInputBinding } from './metadata-input'
import { stableJSON } from './provider'
import { recoveredPanel } from './recovered-panel'
import {
  consumeRecovered,
  recoveredLabel,
  RECOVERED_CHANGED,
  type RecoveredItem,
} from './recovered-work'
import type { DocumentType } from './shell'
import type { ShellCommands } from './shell-commands'
import type { ShellContext } from './shell-context'
import type { ShellMetadata } from './shell-metadata'
import type { SidePanels } from './shell-panels'
import { railButton } from './side-rail'
import { documentType, settingsFor } from './state'

/** The label of a recovered input, in the vocabulary of the document type. */
export function recoveredItemLabel(kind: DocumentType, item: RecoveredItem) {
  const target = item.payload.target
  if (item.kind === 'field' && target?.kind === 'metadata')
    return kind.fields.find((f) => f.field === target.field)?.label || recoveredLabel(item)
  const context = item.payload.context
  if (item.kind === 'form' && context?.kind === 'record')
    return (
      kind.structured?.panels?.find((p) => p.id === context.panel)?.itemLabel ||
      kind.structured?.collections.find((c) => c.field === context.collection)?.itemLabel ||
      recoveredLabel(item)
    )
  return recoveredLabel(item)
}

export type ShellRecovered = ReturnType<typeof shellRecovered>

/** Inputs recovered after a crash, reopened where they were typed. */
export function shellRecovered(
  ctx: ShellContext,
  side: HTMLElement,
  panels: SidePanels,
  metadata: ShellMetadata,
) {
  const { provider, options } = ctx
  /** Shows the recovered input in its original control; false when it no longer fits. */
  const reopen = (item: RecoveredItem) => {
    const kind = documentType(provider)
    if (item.kind === 'form') {
      const context = item.payload.context
      if (context?.kind === 'bibliography')
        return resumeBibliography(provider, ctx.editor, ctx.transport, item)
      if (context?.kind === 'record') {
        const definition = kind.structured?.panels?.find(
          (panel) =>
            panel.collection === context.collection &&
            (!context.panel || panel.id === context.panel),
        )
        if (!definition) return false
        metadata.tab = definition.id
        metadata.render()
        return metadata.resumeRecord(item)
      }
      if (context?.kind === 'band') {
        if (context.root?.kind === 'record') {
          const definition = kind.structured?.panels?.find(
            (panel) =>
              panel.collection === context.root.collection &&
              (!panel.fields || panel.fields.includes(context.root.field)),
          )
          if (definition) {
            metadata.tab = definition.id
            metadata.render()
          }
        }
        return reopenBand(provider, item)
      }
    }
    if (item.kind === 'field') {
      const { target, value, base } = item.payload
      let read: (() => unknown) | undefined
      if (target?.kind === 'metadata') {
        const field = kind.fields.find((field) => field.field === target.field)
        if (!field) return false
        read = () => settingsFor(provider).metadata.fields[field.parameter]
        metadata.tab =
          kind.groups.find((group) =>
            group.sections.some((section) => section.fields.includes(target.field)),
          )?.id || field.group
      } else if (target?.kind === 'option') {
        read = () => settingsFor(provider).metadata.fields.options[target.name]
        metadata.tab = 'general'
      } else if (target?.kind === 'model') {
        read = () => ({
          modelId: settingsFor(provider).modelId,
          template: settingsFor(provider).template,
        })
        metadata.tab = 'general'
      }
      if (!read || stableJSON(read()) !== stableJSON(base)) return false
      metadata.render()
      const input = [
        ...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
          '#metadata-content input, #metadata-content textarea, #metadata-content select',
        ),
      ].find(
        (control) =>
          !control.disabled &&
          !control.closest('[hidden]') &&
          stableJSON(metadataInputBinding(control)?.target) === stableJSON(target),
      )
      if (!input) return false
      metadata.editor.restore(
        target,
        value,
        read,
        () => consumeRecovered(provider, item.id),
        item.id,
        item.payload.resources,
      )
      input.focus()
      return true
    }
    return false
  }
  function show() {
    panels.releaseRecovered()
    panels.adoptRecovered(
      recoveredPanel(
        panels.show(tr('Saisies à reprendre'), 'ui.recovered'),
        provider,
        reopen,
        options.isTauri,
        (item) => recoveredItemLabel(documentType(provider), item),
      ),
    )
  }
  const button = railButton(tr('Saisies à reprendre'), 'history', 'ui.recovered')
  button.onclick = show
  return {
    show,
    register({ add, requiresDocument }: ShellCommands) {
      add('ui.recovered', tr('Saisies à reprendre'), show)
      requiresDocument.add('ui.recovered')
      side.prepend(button)
    },
    /** Tells a document opened from a recovered copy apart from its original file. */
    noticeRecoveredDocument(context: HTMLElement) {
      if (!provider.recoveredName) return
      const notice = document.createElement('div')
      notice.className = 'recovered-document-line'
      notice.textContent = tr('Document récupéré · copie indépendante')
      context.before(notice)
    },
    /** Shows the rail button only while recovered inputs remain. */
    track() {
      document.addEventListener(RECOVERED_CHANGED, (event) => {
        if ((event as CustomEvent).detail === provider)
          button.hidden = !provider.recoveredItems.length
      })
      button.hidden = !provider.recoveredItems.length
    },
  }
}

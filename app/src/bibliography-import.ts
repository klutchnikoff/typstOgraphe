import { bibliographyQuery } from './bibliography-query'
import type { Provider, Transport } from './provider'
import type { Source, Report, Decision, Notice } from './bibliography-types'
import { tr } from './i18n'
import { diagnosticText, fieldLabel } from './bibliography-fields'
export interface ImportDraft {
  source: Source
  report: Report
  decisions: Decision[]
  ready: string
}
export function refreshImportPlan(root: HTMLElement, plan: Report['plan']) {
  for (const section of root.querySelectorAll<HTMLElement>('[data-incoming]')) {
    const item = plan.items.find((i) => i.incoming === section.dataset.incoming)
    const label = section.querySelector('.bibliography-plan-action')
    if (item && label) label.textContent = planLabel(item.action)
  }
}
function planLabel(action: string) {
  return tr(
    (
      {
        new: 'Nouvelle notice',
        unchanged: 'Notice conservée',
        enrich: 'Notice enrichie',
        conflict: 'Décision nécessaire',
        skip: 'Notice ignorée',
      } as Record<string, string>
    )[action] || 'Vérifier cette notice.',
  )
}
export function importReview(
  root: HTMLElement,
  draft: ImportDraft,
  existing: Notice[],
  changed: () => void,
) {
  const heading = document.createElement('h3')
  heading.textContent = draft.source.name
  const notices = document.createElement('p')
  notices.textContent = tr('{count} notices', { count: draft.report.records.length })
  const original = document.createElement('details'),
    summary = document.createElement('summary'),
    source = document.createElement('pre')
  summary.textContent = tr('Source originale (lecture seule)')
  source.textContent = draft.source.contents
  original.append(summary, source)
  root.append(heading, notices)
  for (const item of draft.report.plan.items) {
    const record = draft.report.records.find((r) => r.id === item.incoming)!
    const section = document.createElement('section')
    section.className = 'bibliography-import-item'
    section.dataset.incoming = item.incoming
    const title = document.createElement('strong')
    title.textContent = record.key
    const action = document.createElement('p')
    action.className = 'bibliography-plan-action'
    action.textContent = planLabel(item.action)
    const data = document.createElement('p')
    const summary = draft.report.summaries?.find((s) => s.record_id === record.id)
    data.textContent = summary
      ? `${summary.title} · ${summary.authors} · ${summary.year}`
      : typeof record.entry.title === 'string'
        ? record.entry.title
        : record.key
    const select = document.createElement('select')
    select.setAttribute('aria-label', `${tr('Décision')} · ${record.key}`)
    const decision = draft.decisions.find((d) => d.incoming === item.incoming)
    select.add(new Option(tr('Selon le rapprochement'), 'auto'))
    select.add(new Option(tr('Conserver séparément'), 'distinct'))
    select.add(new Option(tr('Même référence que…'), 'same'))
    select.add(new Option(tr('Ignorer cette notice'), 'skip'))
    select.value = decision?.kind || 'auto'
    const key = document.createElement('input')
    key.setAttribute('aria-label', `${tr('Clé de citation')} · ${record.key}`)
    key.value = decision?.kind === 'distinct' ? decision.key : record.key
    const target = document.createElement('select')
    target.setAttribute('aria-label', `${tr('Notice correspondante')} · ${record.key}`)
    target.add(new Option('—', ''))
    for (const candidate of [...existing, ...draft.report.records].filter(
      (r) => r.id !== item.incoming,
    )) {
      target.add(new Option(candidate.key, candidate.id))
    }
    if (decision?.kind === 'same') target.value = decision.target
    const update = () => {
      key.hidden = select.value !== 'distinct'
      target.hidden = select.value !== 'same'
      draft.decisions = draft.decisions.filter((d) => d.incoming !== item.incoming)
      if (select.value === 'skip') draft.decisions.push({ kind: 'skip', incoming: item.incoming })
      if (select.value === 'distinct')
        draft.decisions.push({ kind: 'distinct', incoming: item.incoming, key: key.value })
      if (select.value === 'same')
        draft.decisions.push({ kind: 'same', incoming: item.incoming, target: target.value })
      draft.ready = ''
      changed()
    }
    key.hidden = select.value !== 'distinct'
    target.hidden = select.value !== 'same'
    select.onchange = update
    key.oninput = update
    target.onchange = update
    section.append(title, data, action, select, key, target)
    if (item.added_fields.length || item.differing_fields.length) {
      const info = document.createElement('p')
      info.textContent = [
        item.added_fields.length
          ? `${tr('Champs complétés')} : ${item.added_fields.map(fieldLabel).join(', ')}`
          : '',
        item.differing_fields.length
          ? `${tr('Valeurs existantes conservées')} : ${item.differing_fields.map(fieldLabel).join(', ')}`
          : '',
      ]
        .filter(Boolean)
        .join(' · ')
      section.append(info)
    }
    root.append(section)
  }
  const diagnostics = document.createElement('ul')
  diagnostics.className = 'bibliography-diagnostics'
  for (const diagnostic of draft.report.diagnostics) {
    const row = document.createElement('li')
    row.textContent = `${diagnostic.key} · ${diagnosticText(diagnostic.code)}${diagnostic.field ? ` (${fieldLabel(diagnostic.field)})` : ''}`
    diagnostics.append(row)
  }
  root.append(diagnostics, original)
}
export async function reviewImport(provider: Provider, transport: Transport, draft: ImportDraft) {
  return bibliographyQuery<{ plan: Report['plan'] }>(
    provider,
    transport,
    'bibliography.review_import',
    {
      source: draft.source,
      expected: draft.report.library_fingerprint,
      decisions: draft.decisions,
    },
  )
}

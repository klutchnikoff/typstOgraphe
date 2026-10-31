import { tr } from './i18n'
import type { ComparisonReport, ComparisonSide } from './comparison-types'

export function comparisonLabels(report: ComparisonReport) {
  const names = {
    before: report.before_name || tr('Avant'),
    after: report.after_name || tr('Après'),
  }
  switch (report.context) {
    case 'version':
      return {
        choice: { before: tr('Reprendre ce passage'), after: tr('Garder l’actuel') },
        all: { before: tr('Tout reprendre'), after: tr('Tout conserver') },
        headings: [tr('Avant · Version conservée'), tr('Après · Document courant figé')],
        name: (side: ComparisonSide) =>
          side === 'before' ? tr('Reprendre ce passage') : tr('Garder le passage actuel'),
        allName: (side: ComparisonSide) =>
          side === 'before'
            ? tr('Tout reprendre de cette version')
            : tr('Tout conserver du document courant'),
      }
    case 'proposal':
      return {
        choice: { before: tr('Refuser'), after: tr('Accepter') },
        all: { before: tr('Tout refuser'), after: tr('Tout accepter') },
        headings: [tr('Avant · État de départ'), tr('Après · Proposition')],
        name: (side: ComparisonSide) =>
          side === 'before' ? tr('Refuser ce changement') : tr('Accepter ce changement'),
        allName: (side: ComparisonSide) =>
          side === 'before' ? tr('Tout refuser') : tr('Tout accepter'),
      }
    case 'historical':
      return {
        choice: { before: tr('Choisir cette version'), after: tr('Choisir cette version') },
        all: { before: tr('Tout choisir ici'), after: tr('Tout choisir ici') },
        headings: [tr('Avant'), tr('Après')],
        name: (side: ComparisonSide) => tr('Choisir dans « {name} »', { name: names[side] }),
        allName: (side: ComparisonSide) =>
          tr('Tout choisir dans « {name} »', { name: names[side] }),
      }
    case 'composition':
      return {
        choice: { before: tr('Garder l’actuel'), after: tr('Reprendre ce passage') },
        all: { before: tr('Tout conserver'), after: tr('Tout reprendre') },
        headings: [tr('Avant · Document courant figé'), tr('Après · Composition préparée')],
        name: (side: ComparisonSide) =>
          side === 'before'
            ? tr('Garder le passage actuel')
            : tr('Reprendre ce passage de la composition'),
        allName: (side: ComparisonSide) =>
          side === 'before'
            ? tr('Tout conserver du document courant')
            : tr('Tout reprendre de la composition'),
      }
  }
}
export function comparisonDecision(
  report: ComparisonReport,
  choices: ComparisonSide[],
  locale: string,
) {
  const before = choices.filter((c) => c === 'before').length
  const after = choices.length - before
  const plural = new Intl.PluralRules(locale)
  const count = (value: number, one: string, other: string) =>
    tr(plural.select(value) === 'one' ? one : other, {
      count: new Intl.NumberFormat(locale).format(value),
    })
  if (report.context === 'historical')
    return {
      label: tr('Comparer la composition au document courant'),
      summary: tr('La composition reste privée avant sa comparaison au document courant.'),
      disabled: false,
    }
  if (report.context === 'version')
    return {
      label: before
        ? count(before, 'Reprendre {count} passage', 'Reprendre {count} passages')
        : tr('Aucun passage à reprendre'),
      summary: [
        count(before, '{count} passage à reprendre', '{count} passages à reprendre'),
        count(after, '{count} passage conservé', '{count} passages conservés'),
      ].join(' · '),
      disabled: before === 0,
    }
  const retained = count(after, '{count} changement accepté', '{count} changements acceptés')
  const refused = count(before, '{count} changement écarté', '{count} changements écartés')
  return {
    label: after
      ? count(after, 'Appliquer {count} changement', 'Appliquer {count} changements')
      : report.context === 'proposal'
        ? choices.length
          ? tr('Refuser la proposition')
          : tr('Terminer sans modification')
        : tr('Conserver le document courant'),
    summary: `${retained} · ${refused}`,
    disabled: false,
  }
}

export function comparisonOutcome(report: ComparisonReport, choices: ComparisonSide[]) {
  if (report.context === 'proposal' && !choices.includes('after'))
    return tr(
      choices.length
        ? 'Proposition refusée. Le document est inchangé.'
        : 'Proposition terminée sans modification.',
    )
  if (report.context === 'composition' && !choices.includes('after'))
    return tr('Document courant conservé.')
  return tr('Choix appliqués. Vous pouvez enregistrer le document.')
}

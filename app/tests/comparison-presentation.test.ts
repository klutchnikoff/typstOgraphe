import { afterEach, expect, it } from 'vitest'
import { comparisonDecision, comparisonLabels } from '../src/comparison-presentation'
import { initializeUI, tr } from '../src/i18n'
import type { ComparisonContext, ComparisonReport, ComparisonSide } from '../src/comparison-types'
const report = (context: ComparisonContext): ComparisonReport => ({
  context,
  guard: { document: 'doc', revision: 1 },
  source: context === 'version' || context === 'historical' ? 'before' : 'after',
  groups: [],
  stale: false,
  before_name: 'A',
  after_name: 'B',
})
afterEach(() => initializeUI({}))
it.each([
  ['version', ['after', 'after'], 'Aucun passage à reprendre', true],
  ['version', ['before', 'after'], 'Reprendre 1 passage', false],
  ['version', ['before', 'before'], 'Reprendre 2 passages', false],
  ['proposal', ['after', 'after'], 'Appliquer 2 changements', false],
  ['proposal', ['before', 'after'], 'Appliquer 1 changement', false],
  ['proposal', ['before', 'before'], 'Refuser la proposition', false],
  ['proposal', [], 'Terminer sans modification', false],
  ['historical', [], 'Comparer la composition au document courant', false],
  ['composition', ['after'], 'Appliquer 1 changement', false],
  ['composition', ['before'], 'Conserver le document courant', false],
] as [ComparisonContext, ComparisonSide[], string, boolean][])(
  '%s announces its actual decision for %j',
  (context, choices, label, disabled) => {
    expect(comparisonDecision(report(context), choices, 'fr')).toMatchObject({ label, disabled })
  },
)
it('keeps acceptance and conservation vocabulary distinct from column colors', () => {
  expect(comparisonLabels(report('version')).choice).toEqual({
    before: 'Reprendre ce passage',
    after: 'Garder l’actuel',
  })
  expect(comparisonLabels(report('proposal')).choice).toEqual({
    before: 'Refuser',
    after: 'Accepter',
  })
  expect(comparisonLabels(report('historical')).allName('before')).toBe('Tout choisir dans « A »')
  expect(comparisonLabels(report('composition')).headings).toEqual([
    'Avant · Document courant figé',
    'Après · Composition préparée',
  ])
})
it('formats translated templates without altering substituted names and uses the interface locale for plural forms', () => {
  initializeUI({
    'Appliquer {count} changements': 'Apply {count} changes',
    '{count} changements acceptés': '{count} changes accepted',
    '{count} changements écartés': '{count} changes rejected',
    'Choisir dans « {name} »': 'Choose from “{name}”',
  })
  expect(comparisonDecision(report('proposal'), ['after', 'after'], 'en').label).toBe(
    'Apply 2 changes',
  )
  expect(comparisonDecision(report('proposal'), ['after', 'after'], 'en').summary).toBe(
    '2 changes accepted · 0 changes rejected',
  )
  expect(tr('Choisir dans « {name} »', { name: '{count} <name>' })).toBe(
    'Choose from “{count} <name>”',
  )
})

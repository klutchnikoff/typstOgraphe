import { expect, it, vi } from 'vitest'
import {
  prepareBibliographyPublication,
  type BibliographyQuery,
  type CitationDraft,
} from '../src/bibliography-publication'
import type { Draft, Form, Notice } from '../src/bibliography-types'
import type { ImportDraft } from '../src/bibliography-import'
import { stableJSON } from '../src/provider'

const record: Notice = {
  id: 'notice-1',
  provenance: [],
  key: 'example',
  entry: { type: 'Article', title: 'Original' },
}
const draft = (): Draft => ({
  base: structuredClone(record),
  working: structuredClone(record),
  profile: 'article',
  profiles: ['article'],
  fingerprint: 'original-record',
  values: {},
  key: 'example',
  creating: false,
  fields: [],
})
function query(prepared: Notice) {
  const call = vi.fn(async (operation: string) => {
    if (operation === 'bibliography.prepare_form') return { record: prepared } as Form
    if (operation === 'bibliography.inspect') return { fingerprint: 'current-library' }
    throw new Error('Unexpected query')
  })
  return { call, api: call as BibliographyQuery }
}
it('prepares a guarded replacement without losing identity or touching the private draft', async () => {
  const selected = draft(),
    before = stableJSON(selected)
  const next = { ...record, entry: { ...record.entry, title: 'Edited' } }
  const { api } = query(next)
  expect(await prepareBibliographyPublication(api, selected)).toEqual([
    {
      id: 'bibliography.apply',
      params: {
        expected: 'current-library',
        change: {
          kind: 'replace',
          id: record.id,
          expected: 'original-record',
          record: next,
        },
      },
    },
  ])
  expect(stableJSON(selected)).toBe(before)
})
it('creates and cites in one batch, preserving existing citation items, form and marks', async () => {
  const selected = { ...draft(), creating: true }
  const cite: CitationDraft = {
    target: { path: [0] },
    document: { type: 'doc' },
    position: 1,
    node: {
      type: 'citation',
      marks: [{ type: 'italic' }],
      attrs: {
        items: [{ record_id: 'previous', supplement: 'p. 2' }],
        form: 'prose',
      },
    },
  }
  const { api } = query(record)
  const batch = await prepareBibliographyPublication(api, selected, undefined, cite)
  expect(batch?.map((command) => command.id)).toEqual(['bibliography.apply', 'inline.replace'])
  expect(batch?.[1].params).toMatchObject({
    target: cite.target,
    node: {
      marks: cite.node?.marks,
      attrs: {
        form: 'prose',
        items: [
          { record_id: 'previous', supplement: 'p. 2' },
          { record_id: record.id, supplement: '' },
        ],
      },
    },
  })
  expect(cite.node?.attrs?.items).toHaveLength(1)
})
it('a clean notice does not publish a spurious replacement', async () => {
  expect(await prepareBibliographyPublication(query(record).api, draft())).toEqual([])
})
it('refuses unreviewed imports and keeps the exact reviewed preparation guard', async () => {
  const review: ImportDraft = {
    source: { id: 'source', name: 'references.yaml', format: 'hayagriva', contents: 'original' },
    report: {
      fingerprint: 'prepared',
      library_fingerprint: 'library',
      records: [],
      diagnostics: [],
      plan: { blocked: true, items: [] },
    },
    decisions: [],
    ready: stableJSON([]),
  }
  const api = vi.fn() as BibliographyQuery
  expect(await prepareBibliographyPublication(api, undefined, review)).toBeNull()
  review.report.plan.blocked = false
  review.ready = 'stale'
  expect(await prepareBibliographyPublication(api, undefined, review)).toBeNull()
  review.ready = stableJSON(review.decisions)
  expect(await prepareBibliographyPublication(api, undefined, review)).toEqual([
    {
      id: 'bibliography.apply',
      params: {
        expected: 'library',
        change: {
          kind: 'import',
          source: review.source,
          preparation: 'prepared',
          decisions: [],
        },
      },
    },
  ])
  expect(api).not.toHaveBeenCalled()
})

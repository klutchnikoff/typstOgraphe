import type { BibliographicRecord } from './citation-index'
export type Notice = BibliographicRecord
export interface Diagnostic {
  code: string
  key: string
  field: string | null
  detail: string
}
export interface BrowseEntry {
  id: string
  key: string
  title: string
  names: string
  authors: string
  year: number | null
  profile: string
  incomplete: boolean
}
export interface Browse {
  fingerprint: string
  total: number
  entries: BrowseEntry[]
}
export interface Field {
  path: string
  kind: string
  group: string
  recommended: boolean
  value: unknown
}
export interface Form {
  profile: string
  profiles: string[]
  record: Notice
  fields: Field[]
  patches: unknown[]
  fingerprint: string
  diagnostics: Diagnostic[]
}
export interface Person {
  id: string
  name: string
  given: string
}
export interface Source {
  id: string
  name: string
  format: 'biblatex' | 'hayagriva'
  contents: string
}
export type Decision =
  | { kind: 'skip'; incoming: string }
  | { kind: 'distinct'; incoming: string; key: string }
  | { kind: 'same'; incoming: string; target: string }
export interface Report {
  summaries?: { record_id: string; title: string; authors: string; year: string }[]
  fingerprint: string
  library_fingerprint: string
  records: Notice[]
  diagnostics: Diagnostic[]
  plan: {
    blocked: boolean
    items: {
      incoming: string
      target: string | null
      action: string
      added_fields: string[]
      differing_fields: string[]
      conflicts: unknown[]
    }[]
  }
}
export interface Draft {
  working: Notice
  profile: string
  profiles: string[]
  base: Notice
  fingerprint: string
  values: Record<string, unknown>
  key: string
  creating: boolean
  fields: Field[]
}
export const profiles: Record<string, string> = {
  article: 'Article',
  journal_article: 'Article de revue',
  book: 'Livre',
  book_chapter: 'Chapitre',
  conference_paper: 'Communication publiée',
  report: 'Rapport',
  thesis: 'Mémoire ou thèse',
  web_resource: 'Ressource web',
  preprint: 'Prépublication',
  software: 'Logiciel',
  other: 'Autre',
}

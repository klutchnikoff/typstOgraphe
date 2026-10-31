import type { VersionContent } from './version-types'
export type ComparisonOrigin =
  { kind: 'version'; id: string; against?: string } | { kind: 'proposal'; id: string }
export type ComparisonSide = 'before' | 'after'
export interface ContentHighlight {
  path: number[]
  from: number
  to: number
}
export interface ComparisonGroup {
  id: number
  section: 'body' | 'document'
  before: { start: number; end: number }
  after: { start: number; end: number }
  linked: boolean
  decision: 'pending' | 'applied' | 'refused'
  changes: { kind: 'equal' | 'delete' | 'insert'; text: string }[]
  before_highlights: ContentHighlight[]
  after_highlights: ContentHighlight[]
}
export interface ComparisonReport {
  guard: { document: string; revision: number }
  source: 'before' | 'after'
  groups: ComparisonGroup[]
  stale: boolean
  closed?: boolean
  applied?: boolean
  before_name?: string | null
  after_name?: string | null
  files?: {
    kind: string
    path: string
    before: { bytes: number; hash: string } | null
    after: { bytes: number; hash: string } | null
  }[]
}
export interface ComparisonState {
  id: string
  report: ComparisonReport
  choices?: ComparisonSide[]
  before: VersionContent
  after: VersionContent
}
export interface ComparisonAPI {
  open(origin: ComparisonOrigin): Promise<ComparisonState>
  read(id: string): Promise<Pick<ComparisonState, 'id' | 'report'>>
  apply(id: string, choices: ComparisonSide[]): Promise<Pick<ComparisonState, 'id' | 'report'>>
  close(id: string): Promise<unknown>
  preview(id: string, side: 'before' | 'after'): Promise<{ pages: string[] }>
  listen(changed: () => void): Promise<() => void>
}

import type { ReferenceEntry } from './citation-index'
import type { JSONContent } from '@tiptap/core'
import type { Resource } from './provider'
import type { Settings } from './state'
export interface VersionDetails {
  name: string
  tags: string[]
  description: string | null
  creator: string | null
}
export interface VersionRecord {
  id: string
  created_at: string
  details: VersionDetails
  reason: 'manual' | 'before_restore'
  state: string
}
export interface VersionCatalogue {
  project_id: string
  history: string
  limit: number
  versions: VersionRecord[]
}
export type VersionAction =
  | { kind: 'create'; details: VersionDetails }
  | { kind: 'update'; id: string; details: VersionDetails }
  | { kind: 'delete'; id: string }
export interface VersionContent {
  references?: ReferenceEntry[]
  state: { document: JSONContent; settings: Settings }
  resources: Resource[]
}
export interface VersionAPI {
  comparison?: import('./comparison-types').ComparisonAPI
  list(): Promise<VersionCatalogue>
  read(id: string): Promise<VersionContent>
  preview(id: string): Promise<{ pages: string[] }>
  change(history: string, action: VersionAction): Promise<VersionCatalogue | null>
}

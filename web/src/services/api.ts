export type ProjectRole = 'viewer' | 'editor' | 'approver' | 'admin'

export interface User {
  id: string
  username: string
  is_system_admin: boolean
}

export interface Session {
  user: User
  token: string
  csrf_token: string
  expires_in: number
}

export interface Project {
  id: string
  name: string
  description: string
  role: ProjectRole
  version: number
}

export interface TypeAst {
  kind: string
  min?: number | null
  max?: number | null
  min_length?: number | null
  max_length?: number | null
  item?: TypeAst
  length?: number
  min_items?: number | null
  max_items?: number | null
  schema_id?: string
  mode?: 'hard' | 'soft'
  variants?: Array<{ id: string; name: string; value: number }>
}

export interface FieldDefinition {
  id: string
  name: string
  description: string
  ty: TypeAst
  default: ConfigValue | null
  target: TargetRule
}

export interface TargetRule {
  include: Array<'rust' | 'c_sharp' | 'type_script'>
  audiences: Array<'client' | 'server' | 'editor'>
  rename: Record<string, string>
}

export interface SchemaDefinition {
  id: string
  project_id: string
  name: string
  description: string
  fields: FieldDefinition[]
  target: TargetRule
}

export interface StoredSchema {
  definition: SchemaDefinition
  version: number
  revision_id: string
}

export type ConfigValue = { kind: string; value?: unknown }

export interface ConfigRow {
  id: string
  schema_id: string
  revision_id: string
  values: Record<string, ConfigValue>
}

export interface StoredRow {
  row: ConfigRow
  version: number
}

export interface BuildArtifact {
  path: string
  media_type: string
  sha256: string
  content: number[]
}

export interface BuildRecord {
  id: string
  project_id: string
  target: string
  status: 'queued' | 'running' | 'succeeded' | 'failed'
  input_hash: string | null
  manifest: Record<string, unknown> | null
  artifacts: BuildArtifact[]
}

export interface SyncStatus {
  pending: number
  processed: number
  failed: number
  projected_schemas: number
  projected_rows: number
}

export interface TableView {
  view_id: string
  total_rows: number
  block_size: number
  data_revision: string | null
}

export interface TableViewBlock {
  view_id: string
  block_index: number
  data_revision: string | null
  rows: StoredRow[]
}

const sessionKey = 'datahub.session'

export function loadSession(): Session | null {
  const value = sessionStorage.getItem(sessionKey)
  if (!value) return null
  try {
    return JSON.parse(value) as Session
  } catch {
    sessionStorage.removeItem(sessionKey)
    return null
  }
}

export function storeSession(session: Session | null): void {
  if (session) sessionStorage.setItem(sessionKey, JSON.stringify(session))
  else sessionStorage.removeItem(sessionKey)
}

export async function api<T>(
  path: string,
  options: RequestInit = {},
  session: Session | null = loadSession(),
): Promise<T> {
  const headers = new Headers(options.headers)
  headers.set('Accept', 'application/json')
  if (options.body) headers.set('Content-Type', 'application/json')
  if (session) headers.set('Authorization', `Bearer ${session.token}`)
  if (session && options.method && options.method !== 'GET') {
    headers.set('X-CSRF-Token', session.csrf_token)
  }

  const response = await fetch(`/api/v1${path}`, { ...options, headers })
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as
      | { message?: string; error?: string; details?: unknown }
      | null
    const message = payload?.message ?? payload?.error ?? `请求失败 (${response.status})`
    throw new Error(message)
  }
  return response.json() as Promise<T>
}

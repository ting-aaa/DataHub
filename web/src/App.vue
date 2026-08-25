<script setup lang="ts">
import { ListTable } from '@visactor/vue-vtable'
import { register } from '@visactor/vtable'
import { InputEditor } from '@visactor/vtable-editors'
import { ElMessage } from 'element-plus'
import { computed, onMounted, reactive, ref } from 'vue'

import {
  api,
  loadSession,
  storeSession,
  type AuditEventRecord,
  type BuildArtifact,
  type BuildRecord,
  type ConfigValue,
  type EnvironmentRecord,
  type FieldDefinition,
  type ProjectionPlan,
  type Project,
  type ReleaseRecord,
  type SchemaDefinition,
  type Session,
  type StoredRow,
  type StoredSchema,
  type SyncStatus,
  type TableView,
  type TableViewBlock,
  type TypeAst,
} from './services/api'
import { formatConfigValue, initialDraftValue, parseConfigValue } from './services/config-values'
import { formatBuildSelection, formatFormulaVersion } from './services/display-labels'
import { fetchApiHealth, type HealthPayload } from './services/health'
import { uuidv7 } from './services/uuidv7'

type Audience = 'client' | 'server' | 'editor'
type FieldKind = 'integer' | 'float' | 'string' | 'bool' | 'bytes' | 'date' | 'date_time' | 'enum' | 'list' | 'reference'
type ListItemKind = 'integer' | 'float' | 'string' | 'bool'

interface SchemaFieldDraft {
  key: string
  name: string
  description: string
  fieldType: FieldKind
  referenceSchemaId: string
  enumVariants: string
  listItemType: ListItemKind
  audiences: Audience[]
}

interface CellChangeEvent {
  col: number
  field?: string | number
  recordIndex?: number | number[]
  changedValue: string | number
}

interface CellClickEvent {
  col: number
  row: number
  field?: string | number
}

interface VTableInstance {
  startEditCell: (col: number, row: number) => void
}

interface VTableComponent {
  vTableInstance: VTableInstance | { value: VTableInstance | null } | null
}

interface FormulaDraft {
  key: string
  fieldId: string
  source: string
}

interface StoredFormulaSet {
  schema_revision_id: string
  document: { definitions: Record<string, { field_id: string; source: string }> }
  version: number
}

interface FormulaChange {
  row_id: string
  expected_version: number
  before: { values: Record<string, ConfigValue> }
  after: { values: Record<string, ConfigValue> }
}

interface XlsxArtifact {
  file_name: string
  content_type: string
  content: number[]
}

interface XlsxPreview {
  created: number
  updated: number
  rows: unknown[]
}

function newSchemaFieldDraft(index = 0): SchemaFieldDraft {
  return {
    key: uuidv7(),
    name: index === 0 ? 'id' : `field_${index + 1}`,
    description: '',
    fieldType: 'integer',
    referenceSchemaId: '',
    enumVariants: 'default, enabled',
    listItemType: 'integer',
    audiences: ['client', 'server'],
  }
}

const inputEditor = new InputEditor()
register.editor('datahub-input', inputEditor)

const health = ref<HealthPayload | null>(null)
const session = ref<Session | null>(loadSession())
const requiresBootstrap = ref(false)
const busy = ref(false)
const error = ref('')
const projects = ref<Project[]>([])
const selectedProjectId = ref('')
const schemas = ref<StoredSchema[]>([])
const selectedSchemaId = ref('')
const rows = ref<StoredRow[]>([])
const tableView = ref<TableView | null>(null)
const tableComponent = ref<VTableComponent | null>(null)
const nextVisibleBlock = ref(1)
const loadingNextBlock = ref(false)
const gridActivity = ref('')
const builds = ref<BuildRecord[]>([])
const sync = ref<SyncStatus | null>(null)
const projectionPlans = ref<ProjectionPlan[]>([])
const environments = ref<EnvironmentRecord[]>([])
const releases = ref<ReleaseRecord[]>([])
const auditEvents = ref<AuditEventRecord[]>([])
const buildTarget = ref<'rust' | 'c_sharp' | 'type_script'>('rust')
const buildAudience = ref<'client' | 'server' | 'editor'>('client')
const formulaDrafts = ref<FormulaDraft[]>([{ key: uuidv7(), fieldId: '', source: '' }])
const formulaVersion = ref<number | null>(null)
const formulaRuntime = ref<'native' | 'wasm'>('native')
const formulaPreview = ref<FormulaChange[]>([])
const xlsxContent = ref<number[] | null>(null)
const xlsxPreview = ref<XlsxPreview | null>(null)

const credentials = reactive({ username: '', password: '' })
const projectDraft = reactive({ name: '', description: '' })
const environmentDraft = reactive({ name: 'production', requires_approval: true })
const releaseDraft = reactive({ environment_id: '', build_id: '', version: '1.0.0' })
const auditFilter = reactive({ action: '' })
const schemaDraft = reactive({
  name: '',
  description: '',
  audiences: ['client', 'server', 'editor'] as Audience[],
  fields: [newSchemaFieldDraft()] as SchemaFieldDraft[],
})
const rowDraft = reactive<Record<string, string | number | boolean>>({})
const viewQuery = reactive({
  filterFieldId: '',
  filterValue: '',
  sortFieldId: '',
  sortDirection: 'asc' as 'asc' | 'desc',
})
const tableBlockCache = new Map<number, StoredRow[]>()
const tableBlockRequests = new Map<number, Promise<StoredRow[]>>()

const selectedProject = computed(
  () => projects.value.find((project) => project.id === selectedProjectId.value) ?? null,
)
const selectedSchema = computed(
  () => schemas.value.find((schema) => schema.definition.id === selectedSchemaId.value) ?? null,
)
const canWrite = computed(() =>
  ['editor', 'approver', 'admin'].includes(selectedProject.value?.role ?? ''),
)
const canApprove = computed(() =>
  ['approver', 'admin'].includes(selectedProject.value?.role ?? ''),
)
const statusType = computed(() => (health.value?.status === 'ok' ? 'success' : 'danger'))
const tableOptions = computed(() => ({
  columns: [
    { field: 'id', title: 'Row ID', width: 285 },
    { field: 'version', title: '版本', width: 90 },
    ...(selectedSchema.value?.definition.fields ?? []).map((field) => ({
      field: field.id,
      title: field.name,
      width: 180,
      editor: canWrite.value ? 'datahub-input' : undefined,
    })),
  ],
  records: rows.value.map((stored) => ({
    id: stored.row.id,
    version: stored.version,
    ...Object.fromEntries(
      (selectedSchema.value?.definition.fields ?? []).map((field) => [
        field.id,
        formatConfigValue(stored.row.values[field.id], field.ty),
      ]),
    ),
  })),
  widthMode: 'standard',
  editCellTrigger: 'api',
  keyboardOptions: { editCellOnEnter: true },
}))

function showError(reason: unknown): void {
  error.value = reason instanceof Error ? reason.message : '发生未知错误'
  ElMessage.error(error.value)
}

async function refreshHealth(): Promise<void> {
  try {
    health.value = await fetchApiHealth()
  } catch (reason) {
    health.value = null
    showError(reason)
  }
}

async function initialize(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    await refreshHealth()
    const setup = await api<{ requires_bootstrap: boolean }>('/setup', {}, null)
    requiresBootstrap.value = setup.requires_bootstrap
    if (session.value) await refreshProjects()
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

async function authenticate(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    const path = requiresBootstrap.value ? '/auth/bootstrap' : '/auth/login'
    const result = await api<Session>(
      path,
      { method: 'POST', body: JSON.stringify(credentials) },
      null,
    )
    session.value = result
    storeSession(result)
    credentials.password = ''
    requiresBootstrap.value = false
    await refreshProjects()
    ElMessage.success(requiresBootstrap.value ? '初始化完成' : '登录成功')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

function logout(): void {
  storeSession(null)
  session.value = null
  projects.value = []
  schemas.value = []
  rows.value = []
  selectedProjectId.value = ''
  selectedSchemaId.value = ''
}

async function refreshProjects(): Promise<void> {
  projects.value = await api<Project[]>('/projects', {}, session.value)
  if (!projects.value.some((project) => project.id === selectedProjectId.value)) {
    selectedProjectId.value = projects.value[0]?.id ?? ''
  }
  await refreshSchemas()
  await refreshOperations()
}

async function createProject(): Promise<void> {
  if (!projectDraft.name.trim()) return
  busy.value = true
  try {
    const project = await api<Project>(
      '/projects',
      { method: 'POST', body: JSON.stringify(projectDraft) },
      session.value,
    )
    projectDraft.name = ''
    projectDraft.description = ''
    await refreshProjects()
    selectedProjectId.value = project.id
    await refreshSchemas()
    ElMessage.success('项目已创建')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

async function selectProject(projectId: string): Promise<void> {
  selectedProjectId.value = projectId
  selectedSchemaId.value = ''
  await refreshSchemas()
  await refreshOperations()
}

async function refreshSchemas(): Promise<void> {
  if (!selectedProjectId.value || !session.value) {
    schemas.value = []
    rows.value = []
    return
  }
  schemas.value = await api<StoredSchema[]>(
    `/projects/${selectedProjectId.value}/schemas`,
    {},
    session.value,
  )
  if (!schemas.value.some((schema) => schema.definition.id === selectedSchemaId.value)) {
    selectedSchemaId.value = schemas.value[0]?.definition.id ?? ''
  }
  await Promise.all([refreshRows(), refreshFormulas()])
}

async function createSchema(): Promise<void> {
  if (!selectedProject.value || !schemaDraft.name.trim() || schemaDraft.fields.length === 0) return
  busy.value = true
  try {
    const names = schemaDraft.fields.map((field) => field.name.trim())
    if (names.some((name) => !name)) throw new Error('每个字段都必须有名称')
    if (new Set(names).size !== names.length) throw new Error('字段名称不能重复')
    const definition: SchemaDefinition = {
      id: uuidv7(),
      project_id: selectedProject.value.id,
      name: schemaDraft.name,
      description: schemaDraft.description,
      fields: schemaDraft.fields.map((field) => ({
          id: uuidv7(),
          name: field.name.trim(),
          description: field.description,
          ty: createFieldType(field),
          default: null,
          target: {
            include: ['rust', 'c_sharp', 'type_script'],
            audiences: [...field.audiences],
            rename: {},
          },
        })),
      target: {
        include: ['rust', 'c_sharp', 'type_script'],
        audiences: [...schemaDraft.audiences],
        rename: {},
      },
    }
    const stored = await api<StoredSchema>(
      `/projects/${selectedProject.value.id}/schemas`,
      { method: 'POST', body: JSON.stringify({ definition }) },
      session.value,
    )
    schemaDraft.name = ''
    schemaDraft.description = ''
    schemaDraft.fields.splice(0, schemaDraft.fields.length, newSchemaFieldDraft())
    await refreshSchemas()
    selectedSchemaId.value = stored.definition.id
    await refreshRows()
    ElMessage.success('Schema 已创建并生成不可变 revision')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

function createFieldType(field: SchemaFieldDraft): TypeAst {
  switch (field.fieldType) {
    case 'integer':
      return { kind: 'integer', min: null, max: null }
    case 'float':
      return { kind: 'float', min: null, max: null }
    case 'string':
      return { kind: 'string', min_length: null, max_length: null }
    case 'bool':
      return { kind: 'bool' }
    case 'bytes':
      return { kind: 'bytes' }
    case 'date':
      return { kind: 'date' }
    case 'date_time':
      return { kind: 'date_time' }
    case 'enum':
      return createEnumType(field.enumVariants)
    case 'list':
      return {
        kind: 'list',
        item: createListItemType(field.listItemType),
        min_items: null,
        max_items: null,
      }
    case 'reference':
      if (!field.referenceSchemaId) throw new Error(`${field.name || '引用字段'}需要选择目标 Schema`)
      return { kind: 'reference', schema_id: field.referenceSchemaId, mode: 'hard' }
    default:
      throw new Error(`不支持的字段类型：${field.fieldType}`)
  }
}

function createEnumType(rawVariants: string): TypeAst {
  const names = rawVariants
    .split(',')
    .map((name) => name.trim())
    .filter(Boolean)
  if (names.length === 0) throw new Error('枚举至少需要一个成员')
  if (new Set(names).size !== names.length) throw new Error('枚举成员名称不能重复')
  return {
    kind: 'enum',
    variants: names.map((name, value) => ({ id: uuidv7(), name, value })),
  }
}

function createListItemType(kind: ListItemKind): TypeAst {
  switch (kind) {
    case 'integer':
      return { kind: 'integer', min: null, max: null }
    case 'float':
      return { kind: 'float', min: null, max: null }
    case 'bool':
      return { kind: 'bool' }
    case 'string':
      return { kind: 'string', min_length: null, max_length: null }
  }
}

function addSchemaField(): void {
  schemaDraft.fields.push(newSchemaFieldDraft(schemaDraft.fields.length))
}

function removeSchemaField(index: number): void {
  if (schemaDraft.fields.length > 1) schemaDraft.fields.splice(index, 1)
}

function moveSchemaField(index: number, direction: -1 | 1): void {
  const target = index + direction
  if (target < 0 || target >= schemaDraft.fields.length) return
  const [field] = schemaDraft.fields.splice(index, 1)
  if (field) schemaDraft.fields.splice(target, 0, field)
}

async function selectSchema(schemaId: string): Promise<void> {
  selectedSchemaId.value = schemaId
  viewQuery.filterFieldId = ''
  viewQuery.filterValue = ''
  viewQuery.sortFieldId = ''
  gridActivity.value = ''
  await Promise.all([refreshRows(), refreshFormulas()])
}

async function refreshFormulas(): Promise<void> {
  formulaPreview.value = []
  if (!selectedProjectId.value || !selectedSchemaId.value || !session.value) {
    formulaVersion.value = null
    formulaDrafts.value = [{ key: uuidv7(), fieldId: '', source: '' }]
    return
  }
  const stored = await api<StoredFormulaSet | null>(
    `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/formulas`,
    {},
    session.value,
  )
  formulaVersion.value = stored?.version ?? null
  const definitions = stored ? Object.values(stored.document.definitions) : []
  formulaDrafts.value = definitions.length
    ? definitions.map((definition) => ({
        key: uuidv7(),
        fieldId: definition.field_id,
        source: definition.source,
      }))
    : [{ key: uuidv7(), fieldId: '', source: '' }]
}

function addFormula(): void {
  formulaDrafts.value.push({ key: uuidv7(), fieldId: '', source: '' })
}

function removeFormula(index: number): void {
  formulaDrafts.value.splice(index, 1)
  if (!formulaDrafts.value.length) addFormula()
}

async function saveFormulaSet(): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value) return
  try {
    const definitions = formulaDrafts.value
      .filter((draft) => draft.fieldId && draft.source.trim())
      .map((draft) => ({ field_id: draft.fieldId, source: draft.source.trim() }))
    const stored = await api<StoredFormulaSet>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/formulas`,
      {
        method: 'PUT',
        body: JSON.stringify({ definitions, expected_version: formulaVersion.value }),
      },
      session.value,
    )
    formulaVersion.value = stored.version
    formulaPreview.value = []
    ElMessage.success('公式集已保存并生成不可变 revision')
  } catch (reason) {
    showError(reason)
  }
}

async function runFormulas(commit: boolean): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value) return
  try {
    const endpoint = commit ? 'apply' : 'preview'
    const result = await api<FormulaChange[] | StoredRow[]>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/formulas/${endpoint}`,
      { method: 'POST', body: JSON.stringify({ runtime: formulaRuntime.value }) },
      session.value,
    )
    if (commit) {
      await refreshRows()
      formulaPreview.value = []
      ElMessage.success(`公式已原子应用到 ${(result as StoredRow[]).length} 行`)
    } else {
      formulaPreview.value = result as FormulaChange[]
    }
  } catch (reason) {
    showError(reason)
  }
}

async function downloadXlsx(): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value) return
  try {
    const artifact = await api<XlsxArtifact>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/xlsx/export`,
      { method: 'POST' },
      session.value,
    )
    downloadBytes(artifact.content, artifact.content_type, artifact.file_name)
  } catch (reason) {
    showError(reason)
  }
}

async function selectXlsx(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || !selectedProjectId.value || !selectedSchemaId.value) return
  xlsxContent.value = Array.from(new Uint8Array(await file.arrayBuffer()))
  try {
    xlsxPreview.value = await api<XlsxPreview>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/xlsx/preview`,
      { method: 'POST', body: JSON.stringify({ content: xlsxContent.value }) },
      session.value,
    )
  } catch (reason) {
    xlsxPreview.value = null
    showError(reason)
  }
}

async function commitXlsx(): Promise<void> {
  if (!xlsxContent.value || !selectedProjectId.value || !selectedSchemaId.value) return
  try {
    const saved = await api<StoredRow[]>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/xlsx/commit`,
      { method: 'POST', body: JSON.stringify({ content: xlsxContent.value }) },
      session.value,
    )
    await refreshRows()
    xlsxPreview.value = null
    xlsxContent.value = null
    ElMessage.success(`XLSX 已原子提交 ${saved.length} 行`)
  } catch (reason) {
    showError(reason)
  }
}

function resetTableBlocks(): void {
  tableBlockCache.clear()
  tableBlockRequests.clear()
  nextVisibleBlock.value = 1
  loadingNextBlock.value = false
}

async function fetchTableBlock(blockIndex: number): Promise<StoredRow[]> {
  const cached = tableBlockCache.get(blockIndex)
  if (cached) return cached
  const pending = tableBlockRequests.get(blockIndex)
  if (pending) return pending
  const viewId = tableView.value?.view_id
  if (!viewId) return []
  const request = api<TableViewBlock>(
    `/table-views/${viewId}/blocks/${blockIndex}`,
    {},
    session.value,
  ).then((block) => {
    if (tableView.value?.view_id !== viewId) return []
    tableBlockCache.set(blockIndex, block.rows)
    return block.rows
  })
  tableBlockRequests.set(blockIndex, request)
  try {
    return await request
  } finally {
    tableBlockRequests.delete(blockIndex)
  }
}

function hasTableBlock(blockIndex: number): boolean {
  const view = tableView.value
  return Boolean(view && blockIndex * view.block_size < view.total_rows)
}

function prefetchTableBlock(blockIndex: number): void {
  if (hasTableBlock(blockIndex)) void fetchTableBlock(blockIndex).catch(() => undefined)
}

async function appendNextTableBlock(): Promise<void> {
  const blockIndex = nextVisibleBlock.value
  if (loadingNextBlock.value || !hasTableBlock(blockIndex)) return
  loadingNextBlock.value = true
  try {
    const blockRows = await fetchTableBlock(blockIndex)
    const existing = new Set(rows.value.map((stored) => stored.row.id))
    rows.value.push(...blockRows.filter((stored) => !existing.has(stored.row.id)))
    nextVisibleBlock.value += 1
    prefetchTableBlock(nextVisibleBlock.value)
  } catch (reason) {
    showError(reason)
  } finally {
    loadingNextBlock.value = false
  }
}

async function refreshRows(): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value || !session.value) {
    rows.value = []
    tableView.value = null
    resetTableBlocks()
    return
  }
  resetTableBlocks()
  const fields = selectedSchema.value?.definition.fields ?? []
  const filterField = fields.find((field) => field.id === viewQuery.filterFieldId)
  const filters = filterField && viewQuery.filterValue.trim()
    ? [{ field_id: filterField.id, value: parseConfigValue(filterField.ty, viewQuery.filterValue) }]
    : []
  const sort = fields.some((field) => field.id === viewQuery.sortFieldId)
    ? [{ field_id: viewQuery.sortFieldId, direction: viewQuery.sortDirection }]
    : []
  tableView.value = await api<TableView>(
    `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/views`,
    { method: 'POST', body: JSON.stringify({ block_size: 256, filters, sort }) },
    session.value,
  )
  rows.value = await fetchTableBlock(0)
  resetRowDraft()
  prefetchTableBlock(1)
}

async function applyViewQuery(): Promise<void> {
  try {
    await refreshRows()
    gridActivity.value = `视图已应用：${tableView.value?.total_rows ?? 0} 行`
  } catch (reason) {
    showError(reason)
  }
}

async function resetViewQuery(): Promise<void> {
  viewQuery.filterFieldId = ''
  viewQuery.filterValue = ''
  viewQuery.sortFieldId = ''
  viewQuery.sortDirection = 'asc'
  await applyViewQuery()
}

async function createRow(): Promise<void> {
  if (!selectedSchema.value) return
  busy.value = true
  try {
    const values = Object.fromEntries(
      selectedSchema.value.definition.fields.map((field) => [
        field.id,
        parseConfigValue(field.ty, rowDraft[field.id]),
      ]),
    ) as Record<string, ConfigValue>
    await api<StoredRow>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/rows`,
      {
        method: 'POST',
        body: JSON.stringify({
          row: {
            id: uuidv7(),
            schema_id: selectedSchemaId.value,
            revision_id: selectedSchema.value.revision_id,
            values,
          },
        }),
      },
      session.value,
    )
    await refreshRows()
    ElMessage.success('数据行已保存')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

async function updateGridCell(event: CellChangeEvent): Promise<void> {
  const recordIndex = Array.isArray(event.recordIndex) ? event.recordIndex[0] : event.recordIndex
  const stored = typeof recordIndex === 'number' ? rows.value[recordIndex] : undefined
  const schemaFields = selectedSchema.value?.definition.fields ?? []
  const field = schemaFields.find((candidate) => candidate.id === String(event.field ?? ''))
    ?? schemaFields[event.col - 2]
  if (!canWrite.value || !stored || !field) return
  let value: ConfigValue
  try {
    value = parseConfigValue(field.ty, event.changedValue)
  } catch (reason) {
    showError(reason)
    await refreshRows()
    return
  }
  if (JSON.stringify(value) === JSON.stringify(stored.row.values[field.id])) return
  busy.value = true
  try {
    await api<StoredRow>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/rows/${stored.row.id}`,
      {
        method: 'PUT',
        body: JSON.stringify({
          row: {
            ...stored.row,
            values: {
              ...stored.row.values,
              [field.id]: value,
            },
          },
          expected_version: stored.version,
        }),
      },
      session.value,
    )
    await refreshRows()
    gridActivity.value = `${field.name} 已保存（version ${stored.version + 1}）`
    ElMessage.success(`${field.name} 已保存（version ${stored.version + 1}）`)
  } catch (reason) {
    await refreshRows().catch(() => undefined)
    showError(reason)
  } finally {
    busy.value = false
  }
}

function startGridCellEdit(event: CellClickEvent): void {
  gridActivity.value = `已选择第 ${event.row} 行、第 ${event.col} 列`
  const editable = event.col >= 2 && event.col < (selectedSchema.value?.definition.fields.length ?? 0) + 2
  if (!canWrite.value || !editable) return
  const exposed = tableComponent.value?.vTableInstance
  const instance = exposed && 'value' in exposed ? exposed.value : exposed
  gridActivity.value = `正在编辑第 ${event.row} 行、第 ${event.col - 1} 个字段`
  instance?.startEditCell(event.col, event.row)
}

function startFirstRowEdit(): void {
  const exposed = tableComponent.value?.vTableInstance
  const instance = exposed && 'value' in exposed ? exposed.value : exposed
  if (!canWrite.value || rows.value.length === 0 || !instance) return
  gridActivity.value = '正在编辑首行的第一个字段'
  instance.startEditCell(2, 1)
}

function resetRowDraft(): void {
  for (const key of Object.keys(rowDraft)) delete rowDraft[key]
  for (const field of selectedSchema.value?.definition.fields ?? []) {
    rowDraft[field.id] = initialDraftValue(field.ty)
  }
}

function rowInputPlaceholder(field: FieldDefinition): string {
  switch (field.ty.kind) {
    case 'list':
      return '逗号分隔，例如 1, 2, 3'
    case 'reference':
      return '目标 Row ID'
    case 'bytes':
      return 'UTF-8 文本'
    case 'date':
      return 'YYYY-MM-DD'
    case 'date_time':
      return 'ISO 8601 日期时间'
    default:
      return field.name
  }
}

async function refreshOperations(): Promise<void> {
  if (!selectedProjectId.value || !session.value) {
    builds.value = []
    sync.value = null
    projectionPlans.value = []
    environments.value = []
    releases.value = []
    auditEvents.value = []
    return
  }
  ;[builds.value, sync.value, projectionPlans.value, environments.value, releases.value, auditEvents.value] = await Promise.all([
    api<BuildRecord[]>(`/projects/${selectedProjectId.value}/builds`, {}, session.value),
    api<SyncStatus>(`/projects/${selectedProjectId.value}/sync-status`, {}, session.value),
    api<ProjectionPlan[]>(`/projects/${selectedProjectId.value}/projection-plans`, {}, session.value),
    api<EnvironmentRecord[]>(`/projects/${selectedProjectId.value}/environments`, {}, session.value),
    api<ReleaseRecord[]>(`/projects/${selectedProjectId.value}/releases`, {}, session.value),
    api<AuditEventRecord[]>(`/projects/${selectedProjectId.value}/audit?limit=20`, {}, session.value),
  ])
  releaseDraft.environment_id ||= environments.value[0]?.id ?? ''
  releaseDraft.build_id ||= builds.value[0]?.id ?? ''
}

async function searchAudit(): Promise<void> {
  const action = auditFilter.action.trim()
  const query = action ? `?limit=50&action=${encodeURIComponent(action)}` : '?limit=50'
  try {
    auditEvents.value = await api<AuditEventRecord[]>(
      `/projects/${selectedProjectId.value}/audit${query}`,
      {},
      session.value,
    )
  } catch (reason) {
    showError(reason)
  }
}

async function createBuild(): Promise<void> {
  if (!selectedProjectId.value) return
  busy.value = true
  try {
    await api<BuildRecord>(
      `/projects/${selectedProjectId.value}/builds`,
      {
        method: 'POST',
        body: JSON.stringify({ target: buildTarget.value, audience: buildAudience.value }),
      },
      session.value,
    )
    await refreshOperations()
    ElMessage.success('确定性构建已完成')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

async function rebuildProjection(): Promise<void> {
  if (!selectedProjectId.value) return
  busy.value = true
  try {
    await api<SyncStatus>(
      `/projects/${selectedProjectId.value}/sync/resync`,
      { method: 'POST' },
      session.value,
    )
    await refreshOperations()
    ElMessage.success('PostgreSQL 投影已从主数据重建')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

async function createProjectionPlanForSchema(): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value) return
  try {
    await api<ProjectionPlan>(
      `/projects/${selectedProjectId.value}/projection-plans`,
      { method: 'POST', body: JSON.stringify({ schema_id: selectedSchemaId.value }) },
      session.value,
    )
    await refreshOperations()
    ElMessage.success('DDL 计划已生成')
  } catch (reason) {
    showError(reason)
  }
}

async function approveProjectionPlan(plan: ProjectionPlan): Promise<void> {
  try {
    await api<ProjectionPlan>(
      `/projects/${selectedProjectId.value}/projection-plans/${plan.id}/approve`,
      { method: 'POST' },
      session.value,
    )
    await refreshOperations()
  } catch (reason) {
    showError(reason)
  }
}

async function applyProjectionPlan(plan: ProjectionPlan): Promise<void> {
  try {
    await api<ProjectionPlan>(
      `/projects/${selectedProjectId.value}/projection-plans/${plan.id}/apply`,
      { method: 'POST' },
      session.value,
    )
    await refreshOperations()
    ElMessage.success('DDL 计划已应用')
  } catch (reason) {
    showError(reason)
  }
}

async function createEnvironmentRecord(): Promise<void> {
  if (!environmentDraft.name.trim()) return
  try {
    const created = await api<EnvironmentRecord>(
      `/projects/${selectedProjectId.value}/environments`,
      { method: 'POST', body: JSON.stringify(environmentDraft) },
      session.value,
    )
    releaseDraft.environment_id = created.id
    await refreshOperations()
  } catch (reason) {
    showError(reason)
  }
}

async function createReleaseRecord(): Promise<void> {
  if (!releaseDraft.environment_id || !releaseDraft.build_id || !releaseDraft.version) return
  try {
    await api<ReleaseRecord>(
      `/projects/${selectedProjectId.value}/releases`,
      { method: 'POST', body: JSON.stringify(releaseDraft) },
      session.value,
    )
    await refreshOperations()
    ElMessage.success('不可变发布快照已创建')
  } catch (reason) {
    showError(reason)
  }
}

async function transitionRelease(release: ReleaseRecord, action: 'approve' | 'publish'): Promise<void> {
  try {
    await api<ReleaseRecord>(
      `/projects/${selectedProjectId.value}/releases/${release.id}/${action}`,
      { method: 'POST' },
      session.value,
    )
    await refreshOperations()
  } catch (reason) {
    showError(reason)
  }
}

async function rollbackTo(release: ReleaseRecord): Promise<void> {
  const version = `rollback-${release.version}-${Date.now()}`
  try {
    await api<ReleaseRecord>(
      `/projects/${selectedProjectId.value}/environments/${release.environment_id}/rollback`,
      { method: 'POST', body: JSON.stringify({ target_release_id: release.id, version }) },
      session.value,
    )
    await refreshOperations()
    ElMessage.success(`已回滚到 ${release.version} 的制品快照`)
  } catch (reason) {
    showError(reason)
  }
}

function downloadArtifact(artifact: BuildArtifact): void {
  downloadBytes(
    artifact.content,
    artifact.media_type,
    artifact.path.split('/').at(-1) ?? 'artifact',
  )
}

function downloadBytes(content: number[], mediaType: string, fileName: string): void {
  const blob = new Blob([new Uint8Array(content)], { type: mediaType })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = fileName
  anchor.click()
  URL.revokeObjectURL(url)
}

onMounted(initialize)
</script>

<template>
  <main v-loading="busy" class="app-shell">
    <header class="topbar">
      <div>
        <span class="eyebrow">Docker-first configuration platform</span>
        <h1>DataHub</h1>
      </div>
      <div class="topbar-actions">
        <el-tag :type="statusType" effect="plain">API {{ health?.status ?? 'unknown' }}</el-tag>
        <template v-if="session">
          <span>{{ session.user.username }}</span>
          <el-button plain @click="logout">退出</el-button>
        </template>
      </div>
    </header>

    <el-alert v-if="error" :title="error" type="error" show-icon @close="error = ''" />

    <section v-if="!session" class="auth-layout">
      <div class="auth-copy">
        <el-tag effect="dark">本地部署 · 无付费依赖</el-tag>
        <h2>{{ requiresBootstrap ? '创建首个管理员' : '欢迎回来' }}</h2>
        <p>
          账户、Session、CSRF、项目权限、Schema revision 与审计记录均保存在本地 PostgreSQL。
        </p>
      </div>
      <el-card class="auth-card" shadow="never">
        <el-form label-position="top" @submit.prevent="authenticate">
          <el-form-item label="用户名">
            <el-input v-model="credentials.username" autocomplete="username" />
          </el-form-item>
          <el-form-item label="密码（至少 12 位）">
            <el-input
              v-model="credentials.password"
              type="password"
              show-password
              autocomplete="current-password"
              @keyup.enter="authenticate"
            />
          </el-form-item>
          <el-button
            type="primary"
            native-type="submit"
            :disabled="!credentials.username || credentials.password.length < 12"
          >
            {{ requiresBootstrap ? '初始化并登录' : '登录' }}
          </el-button>
        </el-form>
      </el-card>
    </section>

    <section v-else class="workspace">
      <aside class="rail">
        <div class="section-heading">
          <span>项目</span>
          <el-tag size="small">{{ projects.length }}</el-tag>
        </div>
        <button
          v-for="project in projects"
          :key="project.id"
          class="nav-item"
          :class="{ active: project.id === selectedProjectId }"
          @click="selectProject(project.id)"
        >
          <strong>{{ project.name }}</strong>
          <small>{{ project.role }}</small>
        </button>
        <el-divider />
        <el-input v-model="projectDraft.name" placeholder="新项目名称" />
        <el-input v-model="projectDraft.description" type="textarea" placeholder="项目说明" />
        <el-button type="primary" plain :disabled="!projectDraft.name.trim()" @click="createProject">
          创建项目
        </el-button>
      </aside>

      <section class="content">
        <el-empty v-if="!selectedProject" description="创建或选择一个项目" />
        <template v-else>
          <div class="project-title">
            <div>
              <span class="eyebrow">{{ selectedProject.role }} · v{{ selectedProject.version }}</span>
              <h2>{{ selectedProject.name }}</h2>
              <p>{{ selectedProject.description || '暂无项目说明' }}</p>
            </div>
            <el-tag :type="canWrite ? 'success' : 'info'">
              {{ canWrite ? '可编辑' : '只读' }}
            </el-tag>
          </div>

          <div class="schema-layout">
            <el-card class="schema-list" shadow="never">
              <template #header><strong>Schema</strong></template>
              <button
                v-for="schema in schemas"
                :key="schema.definition.id"
                class="nav-item"
                :class="{ active: schema.definition.id === selectedSchemaId }"
                @click="selectSchema(schema.definition.id)"
              >
                <strong>{{ schema.definition.name }}</strong>
                <small>revision {{ schema.version }}</small>
              </button>
              <el-empty v-if="schemas.length === 0" :image-size="64" description="暂无 Schema" />
            </el-card>

            <el-card class="editor-card" shadow="never">
              <template #header><strong>新增 Schema</strong></template>
              <el-form label-position="top">
                <el-form-item label="Schema 名称">
                  <el-input v-model="schemaDraft.name" :disabled="!canWrite" />
                </el-form-item>
                <el-form-item label="Schema 导出目标（C/S/E）">
                  <el-checkbox-group v-model="schemaDraft.audiences" :disabled="!canWrite">
                    <el-checkbox value="client">C</el-checkbox>
                    <el-checkbox value="server">S</el-checkbox>
                    <el-checkbox value="editor">E</el-checkbox>
                  </el-checkbox-group>
                </el-form-item>
                <el-form-item label="说明">
                  <el-input v-model="schemaDraft.description" :disabled="!canWrite" />
                </el-form-item>

                <div class="field-editor-heading">
                  <strong>字段（{{ schemaDraft.fields.length }}）</strong>
                  <el-button plain :disabled="!canWrite" @click="addSchemaField">添加字段</el-button>
                </div>
                <div
                  v-for="(field, index) in schemaDraft.fields"
                  :key="field.key"
                  class="schema-field-draft"
                >
                  <div class="field-editor-heading">
                    <span>字段 {{ index + 1 }}</span>
                    <div>
                      <el-button text :disabled="index === 0" @click="moveSchemaField(index, -1)">
                        ↑
                      </el-button>
                      <el-button
                        text
                        :disabled="index === schemaDraft.fields.length - 1"
                        @click="moveSchemaField(index, 1)"
                      >
                        ↓
                      </el-button>
                      <el-button
                        text
                        type="danger"
                        :disabled="schemaDraft.fields.length === 1"
                        @click="removeSchemaField(index)"
                      >
                        删除
                      </el-button>
                    </div>
                  </div>
                  <div class="field-editor-grid">
                    <el-form-item label="名称">
                      <el-input v-model="field.name" :disabled="!canWrite" />
                    </el-form-item>
                    <el-form-item label="类型">
                      <el-select v-model="field.fieldType" :disabled="!canWrite">
                        <el-option label="Integer" value="integer" />
                        <el-option label="Float" value="float" />
                        <el-option label="String" value="string" />
                        <el-option label="Bool" value="bool" />
                        <el-option label="Bytes" value="bytes" />
                        <el-option label="Date" value="date" />
                        <el-option label="DateTime" value="date_time" />
                        <el-option label="Enum" value="enum" />
                        <el-option label="Array" value="list" />
                        <el-option label="Hard Ref" value="reference" />
                      </el-select>
                    </el-form-item>
                    <el-form-item v-if="field.fieldType === 'list'" label="数组元素类型">
                      <el-select v-model="field.listItemType" :disabled="!canWrite">
                        <el-option label="Integer" value="integer" />
                        <el-option label="Float" value="float" />
                        <el-option label="String" value="string" />
                        <el-option label="Bool" value="bool" />
                      </el-select>
                    </el-form-item>
                    <el-form-item v-if="field.fieldType === 'enum'" label="枚举成员（逗号分隔）">
                      <el-input v-model="field.enumVariants" :disabled="!canWrite" />
                    </el-form-item>
                    <el-form-item v-if="field.fieldType === 'reference'" label="引用 Schema">
                      <el-select v-model="field.referenceSchemaId" :disabled="!canWrite">
                        <el-option
                          v-for="schema in schemas"
                          :key="schema.definition.id"
                          :label="schema.definition.name"
                          :value="schema.definition.id"
                        />
                      </el-select>
                    </el-form-item>
                    <el-form-item label="字段导出目标（C/S/E）">
                      <el-checkbox-group v-model="field.audiences" :disabled="!canWrite">
                        <el-checkbox value="client">C</el-checkbox>
                        <el-checkbox value="server">S</el-checkbox>
                        <el-checkbox value="editor">E</el-checkbox>
                      </el-checkbox-group>
                    </el-form-item>
                    <el-form-item label="字段说明">
                      <el-input v-model="field.description" :disabled="!canWrite" />
                    </el-form-item>
                  </div>
                </div>

                <el-button
                  type="primary"
                  :disabled="!canWrite || !schemaDraft.name.trim() || schemaDraft.fields.length === 0"
                  @click="createSchema"
                >
                  创建 Schema
                </el-button>
              </el-form>
            </el-card>
          </div>

          <el-card v-if="selectedSchema" class="grid-card" shadow="never">
            <template #header>
              <div class="section-heading">
                <div>
                  <strong>{{ selectedSchema.definition.name }}</strong>
                  <small>
                    {{ selectedSchema.revision_id }} · 已加载 {{ rows.length }} / {{ tableView?.total_rows ?? rows.length }} rows
                  </small>
                </div>
              </div>
            </template>
            <div class="view-toolbar">
              <el-select v-model="viewQuery.filterFieldId" clearable placeholder="筛选字段">
                <el-option
                  v-for="field in selectedSchema.definition.fields"
                  :key="field.id"
                  :label="field.name"
                  :value="field.id"
                />
              </el-select>
              <el-input
                v-model="viewQuery.filterValue"
                clearable
                placeholder="精确匹配值"
                :disabled="!viewQuery.filterFieldId"
                @keyup.enter="applyViewQuery"
              />
              <el-select v-model="viewQuery.sortFieldId" clearable placeholder="排序字段">
                <el-option
                  v-for="field in selectedSchema.definition.fields"
                  :key="field.id"
                  :label="field.name"
                  :value="field.id"
                />
              </el-select>
              <el-select v-model="viewQuery.sortDirection" :disabled="!viewQuery.sortFieldId">
                <el-option label="升序" value="asc" />
                <el-option label="降序" value="desc" />
              </el-select>
              <el-button type="primary" plain @click="applyViewQuery">应用视图</el-button>
              <el-button @click="resetViewQuery">重置</el-button>
            </div>
            <div class="row-draft-grid">
              <label v-for="field in selectedSchema.definition.fields" :key="field.id">
                <span>{{ field.name }}</span>
                <el-switch
                  v-if="field.ty.kind === 'bool'"
                  v-model="rowDraft[field.id]"
                  :disabled="!canWrite"
                />
                <el-select
                  v-else-if="field.ty.kind === 'enum'"
                  v-model="rowDraft[field.id]"
                  :disabled="!canWrite"
                >
                  <el-option
                    v-for="variant in field.ty.variants"
                    :key="variant.id"
                    :label="variant.name"
                    :value="variant.name"
                  />
                </el-select>
                <el-input-number
                  v-else-if="['integer', 'float'].includes(field.ty.kind)"
                  v-model="rowDraft[field.id]"
                  :disabled="!canWrite"
                />
                <el-input
                  v-else
                  v-model="rowDraft[field.id]"
                  :placeholder="rowInputPlaceholder(field)"
                  :disabled="!canWrite"
                />
              </label>
              <el-button type="primary" :disabled="!canWrite" @click="createRow">新增数据行</el-button>
              <el-button :disabled="!canWrite || rows.length === 0" @click="startFirstRowEdit">
                编辑首行
              </el-button>
            </div>
            <p class="grid-hint">单击单元格或选中后按 Enter 编辑；保存使用当前 Row version 做乐观并发检查。</p>
            <p v-if="gridActivity" class="grid-hint">{{ gridActivity }}</p>
            <ListTable
              ref="tableComponent"
              :options="tableOptions"
              :height="460"
              @on-click-cell="startGridCellEdit"
              @on-change-cell-value="updateGridCell"
              @on-scroll-vertical-end="appendNextTableBlock"
            />
            <div v-if="loadingNextBlock" class="block-loading">正在载入下一数据块…</div>
          </el-card>

          <div v-if="selectedSchema" class="m4-layout">
            <el-card shadow="never">
              <template #header>
                <div class="section-heading">
                  <div>
                    <strong>计算字段公式</strong>
                    <small>{{ formatFormulaVersion(formulaVersion) }}</small>
                  </div>
                  <el-button plain :disabled="!canWrite" @click="addFormula">添加公式</el-button>
                </div>
              </template>
              <div
                v-for="(draft, index) in formulaDrafts"
                :key="draft.key"
                class="formula-row"
              >
                <el-select v-model="draft.fieldId" placeholder="计算目标字段">
                  <el-option
                    v-for="field in selectedSchema.definition.fields"
                    :key="field.id"
                    :label="field.name"
                    :value="field.id"
                  />
                </el-select>
                <el-input v-model="draft.source" placeholder="例如 price * quantity" />
                <el-button text type="danger" @click="removeFormula(index)">删除</el-button>
              </div>
              <div class="formula-actions">
                <el-select v-model="formulaRuntime" style="width: 130px">
                  <el-option label="Native" value="native" />
                  <el-option label="WebAssembly" value="wasm" />
                </el-select>
                <el-button type="primary" :disabled="!canWrite" @click="saveFormulaSet">
                  保存公式
                </el-button>
                <el-button @click="runFormulas(false)">预览差异</el-button>
                <el-button type="success" :disabled="!canWrite" @click="runFormulas(true)">
                  原子应用
                </el-button>
              </div>
              <p class="grid-hint">
                预览 {{ formulaPreview.length }} 行；字段重命名后仍按稳定 FieldId 解析和执行。
              </p>
            </el-card>

            <el-card shadow="never">
              <template #header>
                <div class="section-heading">
                  <div>
                    <strong>XLSX 往返</strong>
                    <small>隐藏稳定 ID · 乐观版本 · 全事务提交</small>
                  </div>
                  <el-button @click="downloadXlsx">导出 XLSX</el-button>
                </div>
              </template>
              <label class="xlsx-picker">
                <span>选择 DataHub XLSX 并生成导入预览</span>
                <input type="file" accept=".xlsx" @change="selectXlsx">
              </label>
              <el-alert
                v-if="xlsxPreview"
                :closable="false"
                type="info"
                :title="`预览：新增 ${xlsxPreview.created} 行，更新 ${xlsxPreview.updated} 行`"
              />
              <el-button
                type="success"
                :disabled="!canWrite || !xlsxPreview"
                @click="commitXlsx"
              >
                原子提交导入
              </el-button>
              <p class="grid-hint">
                外部 Schema、过期 revision、缺失公式缓存或任一行版本冲突都会拒绝整批提交。
              </p>
            </el-card>
          </div>

          <div class="operations-layout">
            <el-card shadow="never">
              <template #header>
                <div class="section-heading">
                  <strong>Build 产物</strong>
                  <div class="row-create">
                    <el-select v-model="buildAudience" style="width: 120px">
                      <el-option label="Client (C)" value="client" />
                      <el-option label="Server (S)" value="server" />
                      <el-option label="Editor (E)" value="editor" />
                    </el-select>
                    <el-select v-model="buildTarget" style="width: 150px">
                      <el-option label="Rust" value="rust" />
                      <el-option label="C#" value="c_sharp" />
                      <el-option label="TypeScript" value="type_script" />
                    </el-select>
                    <el-button type="primary" :disabled="!canWrite" @click="createBuild">构建</el-button>
                  </div>
                </div>
              </template>
              <el-collapse v-if="builds.length">
                <el-collapse-item
                  v-for="build in builds"
                  :key="build.id"
                  :title="`${build.target} · ${build.status} · ${build.artifacts.length} artifacts`"
                >
                  <button
                    v-for="artifact in build.artifacts"
                    :key="artifact.path"
                    class="artifact"
                    @click="downloadArtifact(artifact)"
                  >
                    <span>{{ artifact.path }}</span>
                    <small>{{ artifact.sha256.slice(0, 12) }}</small>
                  </button>
                </el-collapse-item>
              </el-collapse>
              <el-empty v-else :image-size="64" description="尚未构建" />
            </el-card>

            <el-card shadow="never">
              <template #header>
                <div class="section-heading">
                  <strong>PostgreSQL 同步</strong>
                  <div>
                    <el-button plain @click="refreshOperations">刷新</el-button>
                    <el-button :disabled="!canWrite" @click="rebuildProjection">完整重建</el-button>
                  </div>
                </div>
              </template>
              <el-descriptions v-if="sync" :column="2" border>
                <el-descriptions-item label="待处理">{{ sync.pending }}</el-descriptions-item>
                <el-descriptions-item label="重试中">{{ sync.retrying }}</el-descriptions-item>
                <el-descriptions-item label="死信">{{ sync.dead_lettered }}</el-descriptions-item>
                <el-descriptions-item label="已处理">{{ sync.processed }}</el-descriptions-item>
                <el-descriptions-item label="Schema 投影">{{ sync.projected_schemas }}</el-descriptions-item>
                <el-descriptions-item label="Row 投影">{{ sync.projected_rows }}</el-descriptions-item>
                <el-descriptions-item label="检查点">{{ sync.checkpoint?.status ?? '尚未创建' }}</el-descriptions-item>
              </el-descriptions>
              <el-divider />
              <div class="section-heading">
                <strong>DDL 计划</strong>
                <el-button :disabled="!canWrite || !selectedSchemaId" @click="createProjectionPlanForSchema">
                  为当前 Schema 生成
                </el-button>
              </div>
              <el-collapse v-if="projectionPlans.length">
                <el-collapse-item
                  v-for="plan in projectionPlans"
                  :key="plan.id"
                  :title="`${plan.status} · ${plan.destructive ? '破坏性' : '兼容'} · ${plan.operations.length} 项`"
                >
                  <pre v-for="operation in plan.operations" :key="operation.sql">{{ operation.sql }}</pre>
                  <el-button
                    v-if="plan.status === 'draft' && plan.destructive"
                    :disabled="!canApprove"
                    @click="approveProjectionPlan(plan)"
                  >
                    审批
                  </el-button>
                  <el-button
                    v-if="(plan.status === 'draft' && !plan.destructive) || plan.status === 'approved'"
                    type="primary"
                    :disabled="!canWrite"
                    @click="applyProjectionPlan(plan)"
                  >
                    应用
                  </el-button>
                </el-collapse-item>
              </el-collapse>
            </el-card>

            <el-card shadow="never">
              <template #header><strong>环境与不可变发布</strong></template>
              <div class="row-create">
                <el-input v-model="environmentDraft.name" placeholder="环境名称" />
                <el-checkbox v-model="environmentDraft.requires_approval">强制审批</el-checkbox>
                <el-button :disabled="!canApprove" @click="createEnvironmentRecord">创建环境</el-button>
              </div>
              <el-divider />
              <div class="row-create">
                <el-select v-model="releaseDraft.environment_id" placeholder="环境">
                  <el-option v-for="environment in environments" :key="environment.id" :label="environment.name" :value="environment.id" />
                </el-select>
                <el-select v-model="releaseDraft.build_id" placeholder="构建">
                  <el-option v-for="build in builds" :key="build.id" :label="formatBuildSelection(build)" :value="build.id" />
                </el-select>
                <el-input v-model="releaseDraft.version" placeholder="版本" />
                <el-button type="primary" :disabled="!canWrite" @click="createReleaseRecord">创建发布</el-button>
              </div>
              <el-collapse v-if="releases.length">
                <el-collapse-item
                  v-for="release in releases"
                  :key="release.id"
                  :title="`${release.version} · ${release.status} · ${release.input_hash.slice(0, 12)}`"
                >
                  <small>Build {{ release.build_id }} · Environment {{ release.environment_id }}</small>
                  <div class="row-create">
                    <el-button v-if="release.status === 'draft'" :disabled="!canApprove" @click="transitionRelease(release, 'approve')">审批</el-button>
                    <el-button v-if="release.status === 'draft' || release.status === 'approved'" :disabled="!canWrite" type="success" @click="transitionRelease(release, 'publish')">发布</el-button>
                    <el-button v-if="release.status === 'published' || release.status === 'superseded'" :disabled="!canApprove" type="warning" @click="rollbackTo(release)">回滚到此版本</el-button>
                  </div>
                </el-collapse-item>
              </el-collapse>
            </el-card>

            <el-card shadow="never">
              <template #header>
                <div class="section-heading">
                  <strong>项目审计</strong>
                  <div class="row-create">
                    <el-input v-model="auditFilter.action" placeholder="按 action 精确筛选" clearable />
                    <el-button @click="searchAudit">检索</el-button>
                  </div>
                </div>
              </template>
              <el-table :data="auditEvents" max-height="360">
                <el-table-column prop="created_at" label="时间" width="210" />
                <el-table-column prop="action" label="Action" width="180" />
                <el-table-column prop="entity_type" label="实体" width="150" />
                <el-table-column prop="entity_id" label="ID" min-width="260" />
                <el-table-column prop="correlation_id" label="Correlation" min-width="260" />
              </el-table>
            </el-card>
          </div>
        </template>
      </section>
    </section>
  </main>
</template>
  type BuildArtifact,
  type BuildRecord,

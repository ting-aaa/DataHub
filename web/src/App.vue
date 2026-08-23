<script setup lang="ts">
import { ListTable } from '@visactor/vue-vtable'
import { ElMessage } from 'element-plus'
import { computed, onMounted, reactive, ref } from 'vue'

import {
  api,
  loadSession,
  storeSession,
  type BuildArtifact,
  type BuildRecord,
  type ConfigValue,
  type Project,
  type SchemaDefinition,
  type Session,
  type StoredRow,
  type StoredSchema,
  type SyncStatus,
  type TableView,
  type TableViewBlock,
} from './services/api'
import { fetchApiHealth, type HealthPayload } from './services/health'
import { uuidv7 } from './services/uuidv7'

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
const builds = ref<BuildRecord[]>([])
const sync = ref<SyncStatus | null>(null)
const buildTarget = ref<'rust' | 'c_sharp' | 'type_script'>('rust')
const buildAudience = ref<'client' | 'server' | 'editor'>('client')

const credentials = reactive({ username: '', password: '' })
const projectDraft = reactive({ name: '', description: '' })
const schemaDraft = reactive({
  name: '',
  description: '',
  fieldName: 'id',
  fieldType: 'integer',
  referenceSchemaId: '',
  audiences: ['client', 'server'] as Array<'client' | 'server' | 'editor'>,
})
const rowDraft = reactive({ value: 0, text: '', checked: false })

const selectedProject = computed(
  () => projects.value.find((project) => project.id === selectedProjectId.value) ?? null,
)
const selectedSchema = computed(
  () => schemas.value.find((schema) => schema.definition.id === selectedSchemaId.value) ?? null,
)
const canWrite = computed(() =>
  ['editor', 'approver', 'admin'].includes(selectedProject.value?.role ?? ''),
)
const statusType = computed(() => (health.value?.status === 'ok' ? 'success' : 'danger'))
const firstField = computed(() => selectedSchema.value?.definition.fields[0] ?? null)
const tableOptions = computed(() => ({
  columns: [
    { field: 'id', title: 'Row ID', width: 285 },
    { field: 'version', title: '版本', width: 90 },
    { field: 'value', title: firstField.value?.name ?? '值', width: 'auto' },
  ],
  records: rows.value.map((stored) => ({
    id: stored.row.id,
    version: stored.version,
    value: displayValue(firstField.value ? stored.row.values[firstField.value.id] : undefined),
  })),
  widthMode: 'standard',
}))

function displayValue(value: ConfigValue | undefined): string {
  if (!value) return '—'
  if (value.value === undefined) return value.kind
  return typeof value.value === 'object' ? JSON.stringify(value.value) : String(value.value)
}

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
  await refreshRows()
}

async function createSchema(): Promise<void> {
  if (!selectedProject.value || !schemaDraft.name.trim() || !schemaDraft.fieldName.trim()) return
  busy.value = true
  try {
    const fieldType = createFieldType()
    if (!fieldType) {
      ElMessage.warning('引用类型需要先选择目标 Schema')
      return
    }
    const definition: SchemaDefinition = {
      id: uuidv7(),
      project_id: selectedProject.value.id,
      name: schemaDraft.name,
      description: schemaDraft.description,
      fields: [
        {
          id: uuidv7(),
          name: schemaDraft.fieldName,
          description: '',
          ty: fieldType,
          default: null,
          target: {
            include: ['rust', 'c_sharp', 'type_script'],
            audiences: [...schemaDraft.audiences],
            rename: {},
          },
        },
      ],
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

function createFieldType() {
  switch (schemaDraft.fieldType) {
    case 'integer':
      return { kind: 'integer', min: null, max: null }
    case 'float':
      return { kind: 'float', min: null, max: null }
    case 'string':
      return { kind: 'string', min_length: null, max_length: null }
    case 'bool':
      return { kind: 'bool' }
    case 'enum':
      return {
        kind: 'enum',
        variants: [
          { id: uuidv7(), name: 'default', value: 0 },
          { id: uuidv7(), name: 'enabled', value: 1 },
        ],
      }
    case 'list':
      return {
        kind: 'list',
        item: { kind: 'integer', min: null, max: null },
        min_items: null,
        max_items: null,
      }
    case 'reference':
      return schemaDraft.referenceSchemaId
        ? { kind: 'reference', schema_id: schemaDraft.referenceSchemaId, mode: 'hard' as const }
        : null
    default:
      return null
  }
}

async function selectSchema(schemaId: string): Promise<void> {
  selectedSchemaId.value = schemaId
  await refreshRows()
}

async function refreshRows(): Promise<void> {
  if (!selectedProjectId.value || !selectedSchemaId.value || !session.value) {
    rows.value = []
    tableView.value = null
    return
  }
  tableView.value = await api<TableView>(
    `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/views`,
    { method: 'POST', body: JSON.stringify({ block_size: 512 }) },
    session.value,
  )
  const block = await api<TableViewBlock>(
    `/table-views/${tableView.value.view_id}/blocks/0`,
    {},
    session.value,
  )
  rows.value = block.rows
}

async function createRow(): Promise<void> {
  if (!selectedSchema.value || !firstField.value) return
  busy.value = true
  try {
    await api<StoredRow>(
      `/projects/${selectedProjectId.value}/schemas/${selectedSchemaId.value}/rows`,
      {
        method: 'POST',
        body: JSON.stringify({
          row: {
            id: uuidv7(),
            schema_id: selectedSchemaId.value,
            revision_id: selectedSchema.value.revision_id,
            values: { [firstField.value.id]: createConfigValue(firstField.value.ty) },
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

async function updateFirstRow(): Promise<void> {
  const stored = rows.value[0]
  if (!stored || !firstField.value) return
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
              [firstField.value.id]: createConfigValue(firstField.value.ty),
            },
          },
          expected_version: stored.version,
        }),
      },
      session.value,
    )
    await refreshRows()
    ElMessage.success('首行已乐观锁更新')
  } catch (reason) {
    showError(reason)
  } finally {
    busy.value = false
  }
}

function createConfigValue(type: SchemaDefinition['fields'][number]['ty']): ConfigValue {
  switch (type.kind) {
    case 'integer':
      return { kind: 'integer', value: rowDraft.value }
    case 'float':
      return { kind: 'float', value: rowDraft.value }
    case 'string':
      return { kind: 'string', value: rowDraft.text }
    case 'bool':
      return { kind: 'bool', value: rowDraft.checked }
    case 'enum':
      return { kind: 'enum', value: type.variants?.[0]?.id }
    case 'list':
      return {
        kind: 'list',
        value: rowDraft.text
          .split(',')
          .map((value) => value.trim())
          .filter(Boolean)
          .map((value) => ({ kind: 'integer', value: Number(value) })),
      }
    case 'reference':
      return {
        kind: 'reference',
        value: { schema_id: type.schema_id, row_id: rowDraft.text.trim() },
      }
    default:
      return { kind: 'null' }
  }
}

async function refreshOperations(): Promise<void> {
  if (!selectedProjectId.value || !session.value) {
    builds.value = []
    sync.value = null
    return
  }
  ;[builds.value, sync.value] = await Promise.all([
    api<BuildRecord[]>(`/projects/${selectedProjectId.value}/builds`, {}, session.value),
    api<SyncStatus>(`/projects/${selectedProjectId.value}/sync-status`, {}, session.value),
  ])
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

function downloadArtifact(artifact: BuildArtifact): void {
  const blob = new Blob([new Uint8Array(artifact.content)], { type: artifact.media_type })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = artifact.path.split('/').at(-1) ?? 'artifact'
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
                <el-form-item label="首个整数字段">
                  <el-input v-model="schemaDraft.fieldName" :disabled="!canWrite" />
                </el-form-item>
                <el-form-item label="字段类型">
                  <el-select v-model="schemaDraft.fieldType" :disabled="!canWrite">
                    <el-option label="Integer" value="integer" />
                    <el-option label="Float" value="float" />
                    <el-option label="String" value="string" />
                    <el-option label="Bool" value="bool" />
                    <el-option label="Enum" value="enum" />
                    <el-option label="Array&lt;Integer&gt;" value="list" />
                    <el-option label="Hard Ref" value="reference" />
                  </el-select>
                </el-form-item>
                <el-form-item v-if="schemaDraft.fieldType === 'reference'" label="引用 Schema">
                  <el-select v-model="schemaDraft.referenceSchemaId" :disabled="!canWrite">
                    <el-option
                      v-for="schema in schemas"
                      :key="schema.definition.id"
                      :label="schema.definition.name"
                      :value="schema.definition.id"
                    />
                  </el-select>
                </el-form-item>
                <el-form-item label="导出目标（C/S/E）">
                  <el-checkbox-group v-model="schemaDraft.audiences" :disabled="!canWrite">
                    <el-checkbox value="client">C</el-checkbox>
                    <el-checkbox value="server">S</el-checkbox>
                    <el-checkbox value="editor">E</el-checkbox>
                  </el-checkbox-group>
                </el-form-item>
                <el-form-item label="说明">
                  <el-input v-model="schemaDraft.description" :disabled="!canWrite" />
                </el-form-item>
                <el-button type="primary" :disabled="!canWrite || !schemaDraft.name" @click="createSchema">
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
                    {{ selectedSchema.revision_id }} · {{ tableView?.total_rows ?? rows.length }} rows
                  </small>
                </div>
                <div class="row-create">
                  <el-switch
                    v-if="firstField?.ty.kind === 'bool'"
                    v-model="rowDraft.checked"
                    :disabled="!canWrite"
                  />
                  <el-input
                    v-else-if="['string', 'list', 'reference'].includes(firstField?.ty.kind ?? '')"
                    v-model="rowDraft.text"
                    :placeholder="firstField?.ty.kind === 'list' ? '1,2,3' : '值'"
                    :disabled="!canWrite"
                  />
                  <el-input-number
                    v-else-if="firstField?.ty.kind !== 'enum'"
                    v-model="rowDraft.value"
                    :disabled="!canWrite"
                  />
                  <el-tag v-else>default</el-tag>
                  <el-button type="primary" :disabled="!canWrite" @click="createRow">新增数据行</el-button>
                  <el-button :disabled="!canWrite || rows.length === 0" @click="updateFirstRow">
                    更新首行
                  </el-button>
                </div>
              </div>
            </template>
            <ListTable :options="tableOptions" :height="Math.max(260, rows.length * 42 + 48)" />
          </el-card>

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
                  <el-button plain @click="refreshOperations">刷新</el-button>
                </div>
              </template>
              <el-descriptions v-if="sync" :column="2" border>
                <el-descriptions-item label="待处理">{{ sync.pending }}</el-descriptions-item>
                <el-descriptions-item label="已处理">{{ sync.processed }}</el-descriptions-item>
                <el-descriptions-item label="Schema 投影">{{ sync.projected_schemas }}</el-descriptions-item>
                <el-descriptions-item label="Row 投影">{{ sync.projected_rows }}</el-descriptions-item>
                <el-descriptions-item label="失败">{{ sync.failed }}</el-descriptions-item>
              </el-descriptions>
            </el-card>
          </div>
        </template>
      </section>
    </section>
  </main>
</template>
  type BuildArtifact,
  type BuildRecord,

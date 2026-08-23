<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import { fetchApiHealth, type HealthPayload } from './services/health'

const health = ref<HealthPayload | null>(null)
const error = ref('')
const loading = ref(true)

const statusType = computed(() => (health.value?.status === 'ok' ? 'success' : 'danger'))

async function refreshHealth() {
  loading.value = true
  error.value = ''
  try {
    health.value = await fetchApiHealth()
  } catch (reason) {
    health.value = null
    error.value = reason instanceof Error ? reason.message : 'Unknown health error'
  } finally {
    loading.value = false
  }
}

onMounted(refreshHealth)
</script>

<template>
  <main class="shell">
    <section class="hero">
      <el-tag effect="dark" round>Docker-first foundation</el-tag>
      <h1>DataHub</h1>
      <p>面向游戏配置的 Schema、数据、编译、发布与同步平台。</p>
    </section>

    <el-card class="status-card" shadow="never" v-loading="loading">
      <template #header>
        <div class="card-header">
          <span>部署状态</span>
          <el-button type="primary" plain @click="refreshHealth">刷新</el-button>
        </div>
      </template>

      <el-alert v-if="error" :title="error" type="error" show-icon :closable="false" />
      <el-descriptions v-else-if="health" :column="1" border>
        <el-descriptions-item label="API">
          <el-tag :type="statusType">{{ health.status }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="服务">{{ health.service }}</el-descriptions-item>
        <el-descriptions-item label="版本">{{ health.version }}</el-descriptions-item>
        <el-descriptions-item label="数据库">PostgreSQL</el-descriptions-item>
      </el-descriptions>
    </el-card>

    <section class="capabilities">
      <article>
        <strong>Schema</strong>
        <span>稳定 ID、类型系统与 Target 裁剪</span>
      </article>
      <article>
        <strong>Config Grid</strong>
        <span>Vue 3、Element Plus 与 VTable</span>
      </article>
      <article>
        <strong>Compiler</strong>
        <span>确定性代码和数据产物</span>
      </article>
    </section>
  </main>
</template>

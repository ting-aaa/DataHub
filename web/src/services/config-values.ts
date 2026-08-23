import type { ConfigValue, TypeAst } from './api'

function requiredText(raw: unknown, label: string): string {
  const value = String(raw ?? '').trim()
  if (!value) throw new Error(`${label}不能为空`)
  return value
}

function listParts(raw: unknown): string[] {
  const value = String(raw ?? '').trim()
  if (!value) return []
  return value
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean)
}

export function parseConfigValue(type: TypeAst, raw: unknown): ConfigValue {
  switch (type.kind) {
    case 'optional':
      return String(raw ?? '').trim() ? parseConfigValue(type.item ?? { kind: 'string' }, raw) : { kind: 'null' }
    case 'bool': {
      if (typeof raw === 'boolean') return { kind: 'bool', value: raw }
      const value = requiredText(raw, '布尔值').toLowerCase()
      if (['true', '1', 'yes'].includes(value)) return { kind: 'bool', value: true }
      if (['false', '0', 'no'].includes(value)) return { kind: 'bool', value: false }
      throw new Error('布尔值只能是 true/false')
    }
    case 'integer': {
      const value = Number(requiredText(raw, '整数'))
      if (!Number.isSafeInteger(value)) throw new Error('请输入安全范围内的整数')
      return { kind: 'integer', value }
    }
    case 'float': {
      const value = Number(requiredText(raw, '浮点数'))
      if (!Number.isFinite(value)) throw new Error('请输入有限浮点数')
      return { kind: 'float', value }
    }
    case 'string':
      return { kind: 'string', value: String(raw ?? '') }
    case 'bytes':
      return { kind: 'bytes', value: Array.from(new TextEncoder().encode(String(raw ?? ''))) }
    case 'date':
      return { kind: 'date', value: requiredText(raw, '日期') }
    case 'date_time':
      return { kind: 'date_time', value: requiredText(raw, '日期时间') }
    case 'enum': {
      const candidate = requiredText(raw, '枚举值')
      const variant = type.variants?.find(
        (item) => item.id === candidate || item.name === candidate || String(item.value) === candidate,
      )
      if (!variant) throw new Error(`未知枚举值：${candidate}`)
      return { kind: 'enum', value: variant.id }
    }
    case 'list':
      return {
        kind: 'list',
        value: listParts(raw).map((part) => parseConfigValue(type.item ?? { kind: 'string' }, part)),
      }
    case 'fixed_array':
      return {
        kind: 'fixed_array',
        value: listParts(raw).map((part) => parseConfigValue(type.item ?? { kind: 'string' }, part)),
      }
    case 'set':
      return {
        kind: 'set',
        value: listParts(raw).map((part) => parseConfigValue(type.item ?? { kind: 'string' }, part)),
      }
    case 'reference':
      return {
        kind: 'reference',
        value: { schema_id: requiredText(type.schema_id, '引用 Schema'), row_id: requiredText(raw, '引用 Row ID') },
      }
    default:
      throw new Error(`Web 编辑器暂不支持 ${type.kind} 类型`)
  }
}

export function formatConfigValue(value: ConfigValue | undefined, type?: TypeAst): string {
  if (!value || value.kind === 'null') return ''
  if (value.kind === 'enum' && type?.kind === 'enum') {
    const variant = type.variants?.find((item) => item.id === value.value)
    return variant?.name ?? String(value.value ?? '')
  }
  if (value.kind === 'reference' && typeof value.value === 'object' && value.value) {
    return String((value.value as { row_id?: unknown }).row_id ?? '')
  }
  if (['list', 'fixed_array', 'set'].includes(value.kind) && Array.isArray(value.value)) {
    return value.value
      .map((item) => formatConfigValue(item as ConfigValue, type?.item))
      .join(', ')
  }
  if (value.kind === 'bytes' && Array.isArray(value.value)) {
    return new TextDecoder().decode(new Uint8Array(value.value as number[]))
  }
  return typeof value.value === 'object' ? JSON.stringify(value.value) : String(value.value ?? '')
}

export function initialDraftValue(type: TypeAst): string | number | boolean {
  switch (type.kind) {
    case 'bool':
      return false
    case 'integer':
    case 'float':
      return 0
    case 'enum':
      return type.variants?.[0]?.name ?? ''
    default:
      return ''
  }
}

import type { BuildRecord } from './api'

export function formatFormulaVersion(version: number | null): string {
  return version === null ? 'FieldId AST · 未保存' : `FieldId AST · formula v${version}`
}

export function formatBuildSelection(build: Pick<BuildRecord, 'target' | 'input_hash'>): string {
  const inputHash = build.input_hash?.slice(0, 8)
  return `${build.target} · ${inputHash ?? '历史构建'}`
}

import { describe, expect, it } from 'vitest'

import { formatBuildSelection, formatFormulaVersion } from './display-labels'

describe('operation display labels', () => {
  it('distinguishes an unsaved formula set from a numbered revision', () => {
    expect(formatFormulaVersion(null)).toBe('FieldId AST · 未保存')
    expect(formatFormulaVersion(3)).toBe('FieldId AST · formula v3')
  })

  it('uses a stable label for legacy builds without an input hash', () => {
    expect(formatBuildSelection({ target: 'rust', input_hash: null })).toBe('rust · 历史构建')
    expect(
      formatBuildSelection({
        target: 'type_script',
        input_hash: '0123456789abcdef',
      }),
    ).toBe('type_script · 01234567')
  })
})

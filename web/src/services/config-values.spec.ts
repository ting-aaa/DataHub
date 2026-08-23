import { describe, expect, it } from 'vitest'

import { formatConfigValue, initialDraftValue, parseConfigValue } from './config-values'

describe('configuration value editing', () => {
  it('parses scalar, enum, list, and reference values', () => {
    expect(parseConfigValue({ kind: 'integer' }, '42')).toEqual({ kind: 'integer', value: 42 })
    expect(parseConfigValue({ kind: 'bool' }, 'false')).toEqual({ kind: 'bool', value: false })

    const enumType = { kind: 'enum', variants: [{ id: 'variant-1', name: 'enabled', value: 1 }] }
    expect(parseConfigValue(enumType, 'enabled')).toEqual({ kind: 'enum', value: 'variant-1' })
    expect(formatConfigValue({ kind: 'enum', value: 'variant-1' }, enumType)).toBe('enabled')

    expect(parseConfigValue({ kind: 'list', item: { kind: 'integer' } }, '1, 2')).toEqual({
      kind: 'list',
      value: [
        { kind: 'integer', value: 1 },
        { kind: 'integer', value: 2 },
      ],
    })
    expect(
      parseConfigValue({ kind: 'reference', schema_id: 'schema-1', mode: 'hard' }, 'row-1'),
    ).toEqual({ kind: 'reference', value: { schema_id: 'schema-1', row_id: 'row-1' } })
  })

  it('rejects invalid edits before sending them to the API', () => {
    expect(() => parseConfigValue({ kind: 'integer' }, '1.5')).toThrow('整数')
    expect(() => parseConfigValue({ kind: 'float' }, 'Infinity')).toThrow('有限')
    expect(() => parseConfigValue({ kind: 'bool' }, 'maybe')).toThrow('true/false')
    expect(() => parseConfigValue({ kind: 'reference', schema_id: 'schema-1' }, '')).toThrow('Row ID')
  })

  it('provides type-aware initial row values', () => {
    expect(initialDraftValue({ kind: 'bool' })).toBe(false)
    expect(initialDraftValue({ kind: 'integer' })).toBe(0)
    expect(initialDraftValue({ kind: 'string' })).toBe('')
  })
})

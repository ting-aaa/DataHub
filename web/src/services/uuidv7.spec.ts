import { describe, expect, it, vi } from 'vitest'

import { uuidv7 } from './uuidv7'

describe('uuidv7', () => {
  it('sets the UUIDv7 version and RFC variant', () => {
    vi.spyOn(crypto, 'getRandomValues').mockImplementation((array) => {
      const bytes = array as Uint8Array
      bytes.fill(0xab)
      return array
    })

    expect(uuidv7(1_700_000_000_000)).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    )
  })

  it('sorts identifiers by their timestamp prefix', () => {
    expect(uuidv7(1_700_000_000_000) < uuidv7(1_700_000_000_001)).toBe(true)
  })
})

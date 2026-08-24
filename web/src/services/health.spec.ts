import { afterEach, describe, expect, it, vi } from 'vitest'

import { fetchApiHealth } from './health'

describe('fetchApiHealth', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('returns the API health payload', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ service: 'datahub-api', status: 'ok', version: '0.1.0' }),
      }),
    )

    await expect(fetchApiHealth()).resolves.toEqual({
      service: 'datahub-api',
      status: 'ok',
      version: '0.1.0',
    })
  })

  it('rejects an unavailable API', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503 }))
    await expect(fetchApiHealth()).rejects.toThrow('503')
  })
})

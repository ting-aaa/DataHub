import { afterEach, describe, expect, it, vi } from 'vitest'

import { api, loadSession, storeSession, type Session } from './api'

const session: Session = {
  user: { id: 'user-1', username: 'admin', is_system_admin: true },
  token: 'bearer-token',
  csrf_token: 'csrf-token',
  expires_in: 3600,
}

describe('DataHub API client', () => {
  afterEach(() => {
    sessionStorage.clear()
    vi.unstubAllGlobals()
  })

  it('round-trips the session in session storage', () => {
    storeSession(session)
    expect(loadSession()).toEqual(session)
    storeSession(null)
    expect(loadSession()).toBeNull()
  })

  it('adds bearer and CSRF headers to mutations', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ saved: true }) })
    vi.stubGlobal('fetch', fetchMock)

    await api('/projects', { method: 'POST', body: '{}' }, session)

    const [url, request] = fetchMock.mock.calls[0] as [string, RequestInit]
    const headers = request.headers as Headers
    expect(url).toBe('/api/v1/projects')
    expect(headers.get('Authorization')).toBe('Bearer bearer-token')
    expect(headers.get('X-CSRF-Token')).toBe('csrf-token')
  })

  it('surfaces the API error message', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 409,
        json: async () => ({ message: 'version conflict' }),
      }),
    )
    await expect(api('/projects', {}, session)).rejects.toThrow('version conflict')
  })
})

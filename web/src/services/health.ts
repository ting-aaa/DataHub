export interface HealthPayload {
  service: string
  status: 'ok' | 'unavailable'
  version: string
}

export async function fetchApiHealth(): Promise<HealthPayload> {
  const response = await fetch('/api/health/ready', {
    headers: { Accept: 'application/json' },
  })
  if (!response.ok) {
    throw new Error(`DataHub API readiness failed with ${response.status}`)
  }
  return response.json() as Promise<HealthPayload>
}

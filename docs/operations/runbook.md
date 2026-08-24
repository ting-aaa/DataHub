# DataHub operator runbook

DataHub v1 is operated with Docker Compose and PostgreSQL. It has no required
cloud account, paid runner, telemetry subscription, or external identity
provider.

## Deploy and upgrade

1. Back up PostgreSQL with `scripts/backup-postgres.ps1`.
2. Pull the reviewed release tag and compare `.env.example` with local config.
3. Run `docker compose config --quiet`.
4. Run `docker compose up --build --detach --wait --wait-timeout 300`.
5. Confirm `/health/live`, `/health/ready`, and `/metrics` through the loopback
   API endpoint. The one-shot `migrate` service applies committed SQLx
   migrations before API/Worker startup.

Never downgrade a database by deleting migration records. Restore the pre-upgrade
backup into a fresh volume and deploy the matching application tag instead.

## Diagnose health and alerts

Use `docker compose ps`, `docker compose logs --since 15m api worker
plugin-host postgres`, and `Invoke-WebRequest http://127.0.0.1:8080/metrics`.
Escalate when readiness is zero, HTTP 5xx grows, a checkpoint is failed, or
dead-letter count is non-zero. Request responses carry `X-Request-ID`; search
the same UUID in structured service logs and project audit history.

The bounded Prometheus text endpoint reports HTTP outcome/latency buckets,
database readiness, outbox pending/retry/dead-letter state, failed checkpoints,
and published releases. The internal plugin host exposes run/trap/quota counters
at `/metrics` on the Compose network. Logs and API errors never include database
URLs, passwords, bearer tokens, CSRF material, or secret-file contents.

## Dead letters and full resync

Inspect `/api/v1/projects/{project_id}/sync-status` and Worker logs. Fix the
missing or invalid canonical aggregate before manually making an event
available again. Never delete a dead letter as a first response: retain it for
audit, create a corrected canonical event, and use
`POST /api/v1/projects/{project_id}/sync/resync` to atomically rebuild generic
projections. A successful rebuild leaves historical dead letters intact and
returns the checkpoint to `ready`.

## Release operations

Create an environment with an explicit approval policy, create a release from a
successful deterministic build, approve it with an approver/admin account, then
publish it. Publishing advances only the environment pointer. Rollback creates
a new immutable release containing the selected historical build hash and
manifest; it never edits the old release or artifact bytes.

## Disaster recovery

Create a custom-format backup:

```powershell
pwsh -NoProfile -File scripts/backup-postgres.ps1 `
  -OutputPath C:\Backups\datahub.dump -ComposeProject datahub
```

Test it against the isolated recovery profile and fresh recovery volume:

```powershell
docker compose --profile recovery up --detach postgres-restore --wait
pwsh -NoProfile -File scripts/restore-postgres.ps1 `
  -InputPath C:\Backups\datahub.dump -ComposeProject datahub
docker compose --profile recovery exec postgres-restore `
  psql -U $env:POSTGRES_USER -d "$($env:POSTGRES_DB)_restore" `
  -c 'SELECT version, success FROM _sqlx_migrations ORDER BY version;'
```

The restore script refuses a non-empty target. After verification, switch the
application's external `DATABASE_URL_FILE` to the recovered database under the
operator's normal change-control process. Keep the old volume until application
readiness, audit history, stable IDs, revisions, rows, formulas, artifacts,
projection state, releases, rollback history and outbox counts are reconciled.

## Known v1 limits

- PostgreSQL is a single-instance Compose service; HA/failover is an operator
  concern outside the v1 bundle.
- Rate limits use PostgreSQL fixed windows and account/session keys, so they are
  consistent across API replicas but intentionally do not trust forwarded IPs.
- The web bundle is intentionally feature-complete rather than code-split; its
  production build reports a non-failing large-chunk advisory.
- Metrics are bounded process counters/gauges. Long-term storage and alerting are
  optional integrations, not required paid bindings.

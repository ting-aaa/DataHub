# Docker deployment

Docker Compose is the supported deployment contract for DataHub. The default
stack runs PostgreSQL, a migration job, API, Worker, plugin host, and web console.

The Rust image build defaults Rustup and Cargo downloads to `rsproxy.cn`, matching
the supported Windows development setup. Override the three public build args
`RUSTUP_DIST_SERVER`, `RUSTUP_UPDATE_ROOT`, and
`CARGO_REGISTRIES_CRATES_IO_INDEX` when another mirror is required; update the
repository `.cargo/config.toml` source replacement when changing Cargo mirrors.

## Local deployment

```powershell
Copy-Item .env.example .env
docker compose config
docker compose up --build --detach --wait
docker compose ps
```

Endpoints:

- Web console: `http://127.0.0.1:3000`
- API liveness: `http://127.0.0.1:8080/health/live`
- API readiness: `http://127.0.0.1:8080/health/ready`
- PostgreSQL: `127.0.0.1:5432`

Only loopback ports are published. Services communicate on the Compose network.
The plugin host is not exposed to the host.

## Configuration and secrets

`.env.example` contains development placeholders. Copy it to the ignored `.env`
file and replace values before use. Do not commit real passwords or connection
strings. Production orchestration must inject secrets through its secret manager.

API, Worker and CLI accept `DATABASE_URL_FILE`; when set, it takes precedence
over `DATABASE_URL` and is read from the mounted Docker/Kubernetes secret file.
The error path is redacted. Authentication/mutation fixed-window budgets are
configured with `DATAHUB_AUTH_RATE_LIMIT`, `DATAHUB_MUTATION_RATE_LIMIT`, and
`DATAHUB_RATE_LIMIT_WINDOW_SECONDS`; invalid or non-positive values stop API
startup.

## Persistence and shutdown

The `datahub_postgres-data` volume persists across `docker compose down`.
PostgreSQL 18 and newer must mount that volume at `/var/lib/postgresql`; its
version-specific data directory is managed inside the mount by the official
image.

```powershell
docker compose down
```

Deleting the volume is destructive and is not part of normal shutdown. A reset
must be an explicit, confirmed development operation.

## Host-native development

For fast feedback, PostgreSQL may remain in Compose while Rust and Vue run on the
host. Set a host `DATABASE_URL`, run the migration CLI, start `datahub-api`, then
run `pnpm web:dev`. This convenience path does not replace container verification.

See the [operator runbook](../operations/runbook.md) for upgrade, metrics,
dead-letter, backup/restore, release and disaster-recovery procedures.

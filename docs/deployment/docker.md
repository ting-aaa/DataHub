# Docker deployment

Docker Compose is the supported deployment contract for DataHub. The default
stack runs PostgreSQL, a migration job, API, Worker, plugin host, and web console.

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

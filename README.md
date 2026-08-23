# DataHub

DataHub is a Docker-first game configuration management and compilation platform.
It uses Rust services, a Vue 3 console, and PostgreSQL as its canonical data store.

## Run the stack

Requirements: Docker Desktop with Docker Compose.

```powershell
Copy-Item .env.example .env
docker compose up --build --detach --wait
Invoke-WebRequest http://127.0.0.1:8080/health/ready
Start-Process http://127.0.0.1:3000
```

Stop services without deleting PostgreSQL data:

```powershell
docker compose down
```

Run the complete local quality gate before merging a branch:

```powershell
pwsh -NoProfile -File scripts/quality-gate.ps1
```

The gate runs Rust and web checks, builds an isolated Docker stack on alternate
loopback ports, verifies HTTP and SQL migrations, and proves PostgreSQL volume
persistence. It does not require a paid CI service.

See [Docker deployment](docs/deployment/docker.md) and the
[architecture overview](docs/architecture/overview.md) for details.

## License

DataHub is licensed under the [MIT License](LICENSE).

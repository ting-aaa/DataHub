# DataHub

DataHub is a Docker-first game configuration management and compilation platform.
It uses Rust services, a Vue 3 console, and PostgreSQL as its canonical data store.

The current implementation includes local Argon2id accounts, bearer/CSRF
sessions, project RBAC, immutable Schema/Data revisions, typed configuration
validation, per-field C/S/E target filtering, multi-field Schema design,
server-filtered VTable blocks with inline optimistic editing, deterministic
Rust/C#/TypeScript plus JSON/CSV/XML/BSON/Protobuf/Lua artifacts, revision-pinned
build manifests with stable Protobuf wire IDs, stable FieldId formulas with
Native/Wasmtime execution, stable-ID XLSX preview/atomic import, and
transactional-outbox projection to PostgreSQL. A WIT Component plugin host adds
hash-pinned exact-version installation with no ambient WASI/network access and
bounded fuel, time, memory, input, and output. Formula, XLSX, build, and sync
state is available through the web console and API. PostgreSQL synchronization
uses isolated exponential retries, dead letters, durable checkpoints, full
rebuilds, stable-ID DDL plans with destructive-change approval, and immutable
environment releases with approval, publishing, and exact-snapshot rollback.

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
loopback ports, verifies HTTP, RBAC, validation, target isolation, build
artifacts, SQL migrations, outbox projection, and PostgreSQL volume persistence.
It also proves Native/Wasm formula parity and full XLSX rollback on a stale-row
conflict, parses the built-in codecs, compares deterministic rebuild hashes, and
compiles generated Rust, C#, and TypeScript. Release approval enforcement,
historical rollback reproducibility, DDL approval, poison-event dead lettering,
checkpoint recovery, and full resync are exercised against the live Docker
stack. It does not require a paid CI service.

See [Docker deployment](docs/deployment/docker.md) and the
[architecture overview](docs/architecture/overview.md) for details.
Plugin authors can start with the [plugin development guide](docs/development/plugins.md).

## License

DataHub is licensed under the [MIT License](LICENSE).

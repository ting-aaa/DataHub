# DataHub architecture

DataHub is a Docker-first modular monolith. The repository produces independently
runnable images while keeping the domain and application boundaries in one Cargo
workspace.

```text
Browser -> Web/Nginx -> DataHub API -> PostgreSQL
                             |
                             +------> Worker
                             +------> Plugin Host
```

PostgreSQL is the canonical store for schema definitions, configuration rows,
revisions, jobs, audit records, and the transactional outbox. Generated artifacts
are accessed through an object-storage abstraction; M0 uses a local filesystem
backend and later deployments can use S3-compatible storage.

The API exposes liveness independently of dependencies and readiness only when
PostgreSQL is reachable. A one-shot migration container must complete before API
and Worker startup.

## Process boundaries

- `datahub-api`: HTTP API and readiness endpoints.
- `datahub-worker`: asynchronous build, import, validation, and sync jobs.
- `datahub-plugin-host`: Wasmtime Component/WIT capability sandbox and exact-version registry.
- `datahub-cli`: one-shot administration and migration commands.
- `web`: Vue console served by an unprivileged Nginx process.

M0 establishes these boundaries without prematurely implementing later domain
milestones.

## Repository layout

```text
DataHub/
├── apps/                    # Deployable Rust process entry points
│   ├── datahub-api/         # HTTP API and health contract
│   ├── datahub-cli/         # One-shot migrations and administration
│   ├── datahub-worker/      # Background job execution
│   └── datahub-plugin-host/ # Isolated plugin process boundary
├── crates/                  # Reusable Rust domain and adapter crates
│   ├── datahub-kernel/      # Domain primitives without infrastructure coupling
│   ├── datahub-auth/        # Argon2id credentials and opaque session tokens
│   ├── datahub-export/      # Deterministic code and data artifacts
│   ├── datahub-formula/     # Stable FieldId AST and Native/Wasmtime evaluation
│   ├── datahub-xlsx/        # Stable-ID XLSX export/import validation
│   └── datahub-persistence-pg/ # PostgreSQL pool and migrations
├── web/                     # Vue 3 administration console
├── migrations/              # Ordered SQLx migrations
├── deploy/docker/           # Production-oriented image definitions
├── docs/                    # Architecture, product, development, and operations
├── .github/                 # CI, dependency updates, and pull-request policy
└── compose.yaml             # Supported local/full-stack deployment contract
```

Domain behavior belongs in `crates/`; `apps/` should remain thin composition
roots. Infrastructure adapters depend on domain crates, never the reverse. Web
code consumes HTTP contracts and does not connect directly to PostgreSQL.

## Synchronization and releases

Each outbox event is isolated in its own transaction. Failed projection events
use bounded exponential retry and become dead letters after five attempts, so a
poison event cannot block later work. Per-project checkpoints and an atomic full
resync rebuild generic projections from canonical data.

Stable FieldIds become generated PostgreSQL column identifiers. Compatible DDL
plans can be applied by editors; column removal and type changes are explicitly
marked destructive and require an approver. Releases copy the deterministic
build hash and manifest into immutable environment history. Publishing advances
one environment pointer, while rollback creates another immutable release that
references and reproduces the chosen historical snapshot.

<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:23:41Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-target-architecture-v1",
  "id": "ARCH-20260823-F3A201",
  "kind": "architecture",
  "next_actions": [
    "Build product capabilities on the verified M0 service and deployment boundaries."
  ],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates",
    "deploy",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "User instructions and accepted development plans in the DataHub planning conversation on 2026-08-24.",
    "Repository audit on 2026-08-24 found no product implementation, so this record originally described the target architecture only.",
    "compose.yaml, Cargo.toml, web/package.json, migrations/0001_bootstrap.sql, and RPT-20260823-44414F."
  ],
  "status": "active",
  "summary": "DataHub implements a Docker Compose full-stack boundary with Rust services, a Vue console, Nginx, and PostgreSQL as the canonical store.",
  "supersedes": [],
  "tags": [
    "architecture",
    "docker",
    "postgresql"
  ],
  "task_id": "",
  "tier": "long",
  "title": "Docker-first DataHub architecture",
  "type_version": 1,
  "updated_at": "2026-08-23T18:44:19Z",
  "valid_as_of": "2026-08-24"
}
-->

# Docker-first DataHub architecture

## Runtime Topology

Compose defines PostgreSQL 18.6, a one-shot SQLx migrator, Rust API, worker and plugin host services, plus an Nginx-served Vue console. PostgreSQL is the canonical store. API and web development ports bind only to loopback; service startup is health-gated.

## Repository Boundaries

The Cargo workspace contains API, CLI, worker, plugin-host, kernel, and PostgreSQL persistence packages. The Vue 3 console uses Element Plus and VTable. Migration 0001 establishes system metadata for readiness and version inspection.

## Container Security

Rust and Nginx runtime images use non-root users and read-only filesystems with narrowly scoped tmpfs mounts. Database and artifact state use named volumes. Multi-stage builds keep build toolchains out of runtime images.

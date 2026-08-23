<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:23:51Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-development-standards-v1",
  "id": "STD-20260823-048D0D",
  "kind": "standard",
  "next_actions": [
    "Enforce these standards in future milestones and extend CI as product capabilities are added."
  ],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Workspace AGENTS.md instruction supplied by the user: use uv manager python env.",
    "User instruction on 2026-08-24: PostgreSQL runs in local Docker Desktop and DataHub is intended for Docker deployment.",
    "compose.yaml, deploy/docker, .github/workflows/ci.yml, and RPT-20260823-44414F."
  ],
  "status": "active",
  "summary": "Use uv for Python, Docker-first deployment artifacts, migration-managed PostgreSQL, secret-safe configuration, and evidence-backed verification.",
  "supersedes": [],
  "tags": [
    "docker",
    "standard",
    "tooling"
  ],
  "task_id": "",
  "tier": "long",
  "title": "DataHub development and deployment standards",
  "type_version": 1,
  "updated_at": "2026-08-23T18:44:19Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub development and deployment standards

## Tooling

Run all Python commands through uv. The M0 baseline pins Rust 1.96 and uses Node 24 with pnpm 11.

## Deployment

Treat Docker images and Compose as first-class delivery artifacts. PostgreSQL schema changes are migration-managed. Bind development ports to loopback, run application containers as non-root with read-only filesystems where practical, and protect durable data with named volumes.

## Security and Verification

Keep credentials out of Git and project memory. Require reproducible Rust/frontend checks, migration validation, image builds, Compose health/smoke checks, and CI evidence before integration.

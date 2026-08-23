<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "high",
  "created_at": "2026-08-23T18:23:41Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-target-architecture-v1",
  "id": "ARCH-20260823-F3A201",
  "kind": "architecture",
  "next_actions": [
    "Materialize the topology in the M0 Docker Compose and workspace foundation."
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
    "Repository audit on 2026-08-24 found no product implementation, so this record describes the target architecture only."
  ],
  "status": "active",
  "summary": "DataHub targets a Docker Compose full-stack topology with Rust services, a Vue console, and PostgreSQL as the canonical store.",
  "supersedes": [],
  "tags": [
    "architecture",
    "docker",
    "postgresql"
  ],
  "task_id": "",
  "tier": "long",
  "title": "Docker-first DataHub target architecture",
  "type_version": 1,
  "updated_at": "2026-08-23T18:23:41Z",
  "valid_as_of": "2026-08-24"
}
-->

# Docker-first DataHub target architecture

DataHub targets a Docker Compose full-stack topology with Rust services, a Vue console, and PostgreSQL as the canonical store.

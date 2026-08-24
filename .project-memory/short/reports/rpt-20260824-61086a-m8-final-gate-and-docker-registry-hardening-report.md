<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-24T15:38:17Z",
  "derived_from": [
    "RPT-20260824-242227"
  ],
  "event_id": "datahub-report-m8-final-gate-docker-registry-v1",
  "id": "RPT-20260824-61086A",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    ".cargo/config.toml",
    "apps",
    "crates/datahub-kernel",
    "deploy/docker/rust.Dockerfile",
    "scripts/quality-gate.ps1"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent post-generation-50 M8 final-gate delta on 2026-08-24.",
    "Curator repository audit of Cargo mirror, Dockerfile, JSON tracing and secret tests.",
    "Curator cargo test --workspace --all-features -- --test-threads=2: 38 tests and doctests passed."
  ],
  "status": "superseded",
  "summary": "M8 final gate passes 38 Rust tests, 10 Web tests, a 142-file secret scan, five images and full recovery acceptance after repository/Docker rsproxy and JSON tracing hardening.",
  "supersedes": [
    "RPT-20260824-242227"
  ],
  "tags": [
    "acceptance",
    "docker",
    "m8",
    "rsproxy",
    "security",
    "tracing"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M8 final gate and Docker registry hardening report",
  "type_version": 1,
  "updated_at": "2026-08-24T15:53:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# M8 final gate and Docker registry hardening report

## Docker Build Reliability

Repository .cargo/config.toml replaces crates.io with the rsproxy-sparse registry and keeps retry behavior under version control. deploy/docker/rust.Dockerfile supplies rsproxy Rustup defaults through build arguments/environment and copies the repository Cargo configuration into the builder. This is necessary because host Rustup mirror variables are not inherited automatically by Docker builds.

An isolated API image proof showed Rustup begin downloads in about six seconds, Cargo explicitly update rsproxy-sparse, and dependencies including sqlx-mysql download through the configured registry. The complete release workspace API image built successfully in two minutes sixteen seconds.

## Structured Tracing and Secret Tests

Service tracing now emits JSON records with request/correlation-ID spans. The configuration suite adds three negative secret-file/value cases covering missing/empty/oversized material and unreadable paths without disclosing secret contents or configured paths. Together with the positive file-precedence test, the kernel now contributes 13 tests.

## Final Canonical Gate

The final scripts/quality-gate.ps1 run exited 0 with 38 Rust tests and doctests, 10 Web tests, a tracked-file secret scan over 142 files, WIT/plugin adversarial tests, all five independently built images, SQLx migrations 0001-0008, the complete runtime/recovery/backup-restore/restart suite and all previous M0-M8 acceptance checks.

The curator independently reran the exact Rust test command and confirmed 38 tests: API 1, auth 3, export 6, formula 4, kernel 13, persistence 2, plugin host 4 and XLSX 5. Quality-gate cleanup removed all isolated quality containers and volumes after the successful run.

## Status

This evidence superseded the earlier 35-test M8 completion snapshot and is itself superseded by the integrated release report RPT-20260824-976431. No paid dependency, secret value or sensitive data was encountered.

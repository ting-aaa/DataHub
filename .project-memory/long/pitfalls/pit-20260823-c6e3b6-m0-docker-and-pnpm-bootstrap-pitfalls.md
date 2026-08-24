<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:44:16Z",
  "derived_from": [],
  "event_id": "datahub-pitfall-m0-docker-pnpm-v1",
  "id": "PIT-20260823-C6E3B6",
  "kind": "pitfall",
  "next_actions": [
    "Preserve these constraints in Compose, CI smoke tests, and deployment documentation."
  ],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    "compose.yaml",
    "deploy/docker",
    "pnpm-workspace.yaml"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent implementation delta for TASK-20260823-9C0927 on 2026-08-24.",
    "compose.yaml lines 5, 16, 66-68, 87-89, 113-115, and 136-141; pnpm-workspace.yaml lines 4-5."
  ],
  "status": "active",
  "summary": "PostgreSQL 18 volume paths, read-only non-root Nginx tmpfs mounts, pnpm 11 esbuild approval, and PowerShell SQL quoting require specific handling.",
  "supersedes": [],
  "tags": [
    "docker",
    "nginx",
    "pnpm",
    "postgresql",
    "powershell"
  ],
  "task_id": "TASK-20260823-9C0927",
  "tier": "long",
  "title": "M0 Docker and pnpm bootstrap pitfalls",
  "type_version": 1,
  "updated_at": "2026-08-23T18:44:16Z",
  "valid_as_of": "2026-08-24"
}
-->

# M0 Docker and pnpm bootstrap pitfalls

## PostgreSQL 18 Data Volume

PostgreSQL 18 rejects the legacy direct mount at /var/lib/postgresql/data in this setup. Mount the named volume at /var/lib/postgresql. The initial incorrect first-run volume was empty and was explicitly recreated after correcting the path; do not delete a populated volume when applying this lesson.

## Read-only Nginx Runtime

Running Nginx as non-root with read_only enabled requires writable tmpfs mounts for /tmp, /var/cache/nginx, and /var/run. Set tmpfs mode 1777 so startup scripts and worker processes can create runtime files without making the image filesystem writable.

## pnpm 11 Build Approval

For the Vue toolchain, pnpm 11 requires allowBuilds with esbuild: true. Replacing it with onlyBuiltDependencies does not satisfy the current install behavior.

## Shell Verification

PowerShell quoting can corrupt inline SQL commands. Prefer server-side constructors such as jsonb_build_object, quote the complete SQL safely, and check $LASTEXITCODE explicitly after commands.

# Local acceptance budgets

The canonical command is `pwsh -NoProfile -File scripts/quality-gate.ps1` from a
clean checkout. It creates fresh named volumes, exercises the complete v1 demo,
restarts the stack, restores a PostgreSQL backup into a second fresh volume and
removes all test containers/volumes in `finally`.

M8 fails when any of these local budgets is exceeded on the supported developer
baseline (Docker Desktop, 4 CPU, 8 GiB available memory):

- 1,024 additional canonical rows are inserted and fully resynchronized within
  30 seconds.
- the first 256-row server-side VTable block returns within 2 seconds.
- two concurrent writes at one expected version yield exactly one success and
  one HTTP 409 within 5 seconds.
- an authentication key returns HTTP 429 after the configured three-request
  quality budget and becomes eligible in at most the configured window plus two
  seconds.
- all five images become healthy within 300 seconds; restart readiness returns
  within 120 seconds.
- backup and restore complete within 120 seconds and durable table counts plus
  deterministic build/release hashes match the source.

These are regression budgets, not throughput claims. The quality database and
credentials are isolated local fixtures and are deleted at the end of the run.

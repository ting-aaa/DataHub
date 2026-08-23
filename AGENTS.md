# Repository Guidelines

- Treat Docker Compose as the supported deployment path; keep host-native runs useful for fast development.
- PostgreSQL is the canonical database. Change schema only through committed SQLx migrations.
- Keep API, worker, plugin-host, and web images independently buildable and health-checked.
- Run Rust formatting, Clippy, and tests before merging. Run frontend lint, type checks, tests, and builds for web changes.
- Never commit credentials or local `.env` files. Commit only `.env.example` placeholders.
- Use `uv` for every Python command and dependency; never invoke an unmanaged Python environment.
- Develop on `feature/*` branches from `develop`. Feature PRs squash into `develop`; release and hotfix PRs merge into `main`.

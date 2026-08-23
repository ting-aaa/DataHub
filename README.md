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

See [Docker deployment](docs/deployment/docker.md) and the
[architecture overview](docs/architecture/overview.md) for details.

## License

DataHub is licensed under the [MIT License](LICENSE).

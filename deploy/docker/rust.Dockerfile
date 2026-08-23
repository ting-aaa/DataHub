FROM rust:1.96-bookworm AS builder

ARG APP
WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY apps ./apps
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --locked --release --bin "${APP}" \
    && mkdir -p /out \
    && cp "target/release/${APP}" /out/datahub

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system datahub \
    && useradd --system --gid datahub --home-dir /nonexistent --shell /usr/sbin/nologin datahub

COPY --from=builder /out/datahub /usr/local/bin/datahub

USER datahub:datahub
ENTRYPOINT ["/usr/local/bin/datahub"]

FROM rust:1.96-bookworm AS builder

ARG RUSTUP_DIST_SERVER=https://rsproxy.cn
ARG RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup
ARG CARGO_REGISTRIES_CRATES_IO_INDEX=sparse+https://rsproxy.cn/index/
ENV RUSTUP_DIST_SERVER=${RUSTUP_DIST_SERVER} \
    RUSTUP_UPDATE_ROOT=${RUSTUP_UPDATE_ROOT} \
    CARGO_REGISTRIES_CRATES_IO_INDEX=${CARGO_REGISTRIES_CRATES_IO_INDEX}

WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY .cargo ./.cargo
COPY apps ./apps
COPY crates ./crates
COPY migrations ./migrations
COPY wit ./wit

RUN cargo build --locked --release --workspace --bins

ARG APP
RUN case "${APP}" in \
        datahub-api|datahub-cli|datahub-plugin-host|datahub-worker) ;; \
        *) echo "unsupported DataHub binary: ${APP}" >&2; exit 2 ;; \
    esac \
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

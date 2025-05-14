# Note: If you update the rust version below, make sure that the debian version which is used there matches the one in the runtime stage
FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
WORKDIR /auditor
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
# Creates a lock file
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install sqlx-cli
RUN cargo install --version=0.8.5 sqlx-cli --no-default-features --features postgres,rustls,sqlite
# Only build project dependencies
COPY --from=planner /auditor/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin auditor

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /auditor

RUN apt-get update -y \
&& apt-get install -y --no-install-recommends openssl ca-certificates curl \
# Clean up
&& apt-get autoremove -y \
&& apt-get clean -y \
&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /auditor/target/release/auditor auditor
COPY --from=builder /usr/local/cargo/bin/sqlx sqlx
COPY --from=builder /auditor/migrations migrations
COPY --from=builder /auditor/containers/auditor/entrypoint.sh entrypoint.sh
COPY --from=builder /auditor/containers/auditor/health_check.sh health_check.sh

COPY auditor/configuration configuration

ENV AUDITOR_ENVIRONMENT=production
HEALTHCHECK --start-period=3s CMD [ "/auditor/health_check.sh" ]
ENTRYPOINT ["./entrypoint.sh"]

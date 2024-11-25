# Note: If you update the rust version below, make sure that the debian version which is used there matches the one in the runtime stage
FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
WORKDIR /auditor
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
# Creates a lock file
# Does not work since we need to do COPY . . after cargo chef cook in the builder
#RUN cargo chef prepare --bin auditor-kubernetes-collector --recipe-path recipe.json
# CI chain might want everything
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install sqlx-cli
RUN cargo install --version=0.8.2 sqlx-cli --no-default-features --features postgres,rustls,sqlite
# Only build project dependencies
COPY --from=planner /auditor/recipe.json recipe.json
RUN cargo chef cook --release --bin auditor-kubernetes-collector --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin auditor-kubernetes-collector

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /auditor
# Mapped users must be able to access the data dir if nothing else is mounted there
RUN mkdir --mode=777 /data

RUN apt-get update -y \
&& apt-get install -y --no-install-recommends openssl ca-certificates \
&& apt-get autoremove -y \
&& apt-get clean -y \
&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /auditor/target/release/auditor-kubernetes-collector auditor-kubernetes-collector

#COPY collectors/kubernetes/config.yml config.yml

ENV AUDITOR_ENVIRONMENT=production
CMD ["./auditor-kubernetes-collector", "config.yaml"]

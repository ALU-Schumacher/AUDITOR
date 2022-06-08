FROM lukemathwalker/cargo-chef:latest-rust-1.60.0 as chef
WORKDIR /auditor-slurm-epilog-collector
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Creates a lock file
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /auditor-slurm-epilog-collector/recipe.json recipe.json
# Only build project dependencies
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin auditor-slurm-epilog-collector

# Runtime stage
FROM debian:bullseye-slim AS runtime

WORKDIR /auditor-slurm-epilog-collector

RUN apt-get update -y \
&& apt-get install -y --no-install-recommends openssl ca-certificates \
# Clean up
&& apt-get autoremove -y \
&& apt-get clean -y \
&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /auditor-slurm-epilog-collector/target/release/auditor-slurm-epilog-collector auditor-slurm-epilog-collector

COPY configuration configuration

ENTRYPOINT ["./auditor-slurm-epilog-collector"]

FROM lukemathwalker/cargo-chef:latest-rust-1.68.0 as chef
WORKDIR /auditor
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Creates a lock file
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /auditor/recipe.json recipe.json
# Only build project dependencies
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin auditor

# Runtime stage
FROM debian:bullseye-slim AS runtime

WORKDIR /auditor

RUN apt-get update -y \
&& apt-get install -y --no-install-recommends openssl ca-certificates \
# Clean up
&& apt-get autoremove -y \
&& apt-get clean -y \
&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /auditor/target/release/auditor auditor

COPY auditor/configuration configuration

ENV AUDITOR_ENVIRONMENT production
ENTRYPOINT ["./auditor"]

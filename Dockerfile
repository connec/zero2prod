FROM rust:1.59.0-alpine AS chef
WORKDIR /app
RUN apk add --no-cache git libc-dev \
  # Work around for a weird bug in DO app platform, which for some reason always fails when cargo
  # tries to init this repo. It's not clear if the digest will ever change though.
  && git config --global init.defaultBranch main && git init /usr/local/cargo/registry/index/github.com-1ecc6299db9ec823 \
  && cargo install cargo-chef


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release


FROM scratch AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/zero2prod zero2prod
ENTRYPOINT ["/app/zero2prod"]

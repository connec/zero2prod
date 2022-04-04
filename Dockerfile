FROM rust:1.59.0-alpine AS chef
WORKDIR app
RUN apk add --no-cache libc-dev && cargo install cargo-chef


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
ENV ADDRESS 0.0.0.0
ENTRYPOINT ["/app/zero2prod"]

#!/usr/bin/env bash

set -euo pipefail

if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed"
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 'Error: sqlx is not installed'
  echo >&2 'Use:'
  echo >&2 '    cargo install sqlx-cli --no-default-features --features "postgres sqlx/runtime-tokio-rustls"'
  echo >&2 'to install it.'
  exit 1
fi

container_name='zero2prod-database'

export PGPORT="${PGPORT:=5432}"
export PGUSER="${PGUSER:=postgres}"
export PGPASSWORD="${PGPASSWORD:=password}"
export PGDATABASE="${PGDATABASE:=newsletter}"

if [ -z "$(docker ps -qf name="$container_name")" ]; then
  container_id="$(docker run \
    -p "$PGPORT":5432 \
    -e POSTGRES_USER="$PGUSER" \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB="$PGDATABASE" \
    --name "$container_name" \
    -d postgres \
    postgres -N 1000)"
fi

until psql -h localhost -c '\q' >/dev/null 2>&1 ; do
  >&2 echo "Postgres is still unavailable - waiting"
  sleep 1
done

export DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@localhost:$PGPORT/$PGDATABASE"
sqlx database create >&2
sqlx migrate run >&2

echo >&2 "Container $container_name is up and running!"

echo "DATABASE_URL=$DATABASE_URL"

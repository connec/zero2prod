#!/usr/bin/env bash

set -euo pipefail

container_name='zero2prod-redis'

redis_port=6379

if [ -z "$(docker ps -qf name="$container_name")" ]; then
  container_id="$(docker run \
    -p "$redis_port:$redis_port" \
    --name "$container_name" \
    -d redis:6)"
fi

echo >&2 "Container $container_name is up and running!"

echo "REDIS_URL=redis://localhost:$redis_port"

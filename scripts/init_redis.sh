#!/usr/bin/env bash
set -x
set -eo pipefail

RUNNINC_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')

if  [[ -n $RUNNINC_CONTAINER ]]; then
    echo >&2 "there is a redis container already running, kill it with"
    echo >&2 "    docker kill ${RUNNINC_CONTAINER}"
    exit 1
fi

docker run \
    -p 6379:6379 \
    -d \
    --name "redis_$(date '+%s')" \
    redis:7

>&2 echo "Postgres has been migrated and ready to go!"

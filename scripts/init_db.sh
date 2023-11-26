#!/usr/bin/env bash
# Enable debugging mode in the script
set -x
# -e instructs the shell to immediately exit if any command exits with a non-zero status (i.e., a command fails).
# -o pipefail extends the -e option to pipelines, so the entire pipeline will return a non-zero status if any part of it fails.
set -eo pipefail

# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER=${POSTGRES_USER:=postgres}
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=ghashy}"
# Check if a custom database name has been set, otherwise default to 'newsletter'
DB_NAME="${POSTGRES_DB:=newsletter}"
# Check if a custom port has been set, otherwise default to '5432'
DB_PORT="${POSTGRES_PORT:=5432}"
# Launch postgres using Docker
docker run \
  --env POSTGRES_USER=${DB_USER} \
  --env POSTGRES_PASSWORD=${DB_PASSWORD} \
  --env POSTGRES_DB=${DB_NAME} \
  --publish "${DB_PORT}":5432 \
  --detach postgres \
  postgres -N 1000
  # ^ Increased maximum number of connections for testing purposes

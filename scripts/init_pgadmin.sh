#!/usr/bin/env bash

docker run \
    --network pg_network \
    --name pgadmin \
    --publish 80:80 \
    --env PGADMIN_DEFAULT_EMAIL=obsidian.musicwork@gmail.com \
    --env PGADMIN_DEFAULT_PASSWORD=ghashy \
    --detach dpage/pgadmin4

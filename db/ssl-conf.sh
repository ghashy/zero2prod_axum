#!/usr/bin/bash

# Define the line to insert
line='hostssl all all all password clientcert=verify-ca'

# Check if the line already exists in pg_hba.conf
if ! grep -qF "$line" /var/lib/postgresql/data/pg_hba.conf; then
    # Insert the line if it doesn't exist
    echo "$line" >> /var/lib/postgresql/data/pg_hba.conf
    echo "Line ${line} added!"
    exit 0
fi

echo "Line ${line} NOT added!"

exec docker-entrypoint.sh "$@"

#!/bin/bash

# SQLx Migration Runner for PostgreSQL

set -e

DATABASE_URL=${DATABASE_URL:-"postgres://postgres:password@localhost:5432/poc_transaction_manager"}

echo "Running sqlx migrations..."
# Mask credentials in logs to avoid exposing secrets
DB_HOST=$(echo "$DATABASE_URL" | sed -E 's|.*@([^/]+)/.*|\1|')
DB_NAME=$(echo "$DATABASE_URL" | sed -E 's|.*/([^?]+).*|\1|')
echo "Target database: $DB_NAME on $DB_HOST"

# Create sqlx binary migration commands
for migration_file in *.sql; do
    if [ -f "$migration_file" ]; then
        echo "Running migration: $migration_file"
        psql "$DATABASE_URL" -f "$migration_file"
    fi
done

echo "All migrations completed successfully!"

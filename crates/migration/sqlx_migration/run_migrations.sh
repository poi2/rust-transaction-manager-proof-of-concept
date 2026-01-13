#!/bin/bash

# SQLx Migration Runner for PostgreSQL

set -e

DATABASE_URL=${DATABASE_URL:-"postgres://postgres:password@localhost:5432/poc_transaction_manager"}

echo "Running sqlx migrations..."
echo "Database URL: $DATABASE_URL"

# Create sqlx binary migration commands
for migration_file in *.sql; do
    if [ -f "$migration_file" ]; then
        echo "Running migration: $migration_file"
        psql "$DATABASE_URL" -f "$migration_file"
    fi
done

echo "All migrations completed successfully!"

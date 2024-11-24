#!/bin/bash

DB_NAME=${DB_NAME:-"postgres"}
DB_USER=${DB_USER:-"postgres"}
DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5432"}
QUERIES_DIR=${QUERIES_DIR:-"."}

# Queries are ordered, make sure to sort them
for sql_file in $(ls "${QUERIES_DIR}"/*.sql | sort); do
    echo "Running: ${sql_file}"
    psql -h "${DB_HOST}" -U "${DB_USER}" -d "${DB_NAME}" -p "${DB_PORT}" -f "${sql_file}" || {
        echo "Failed to run: ${sql_file}"
	exit 1
    }
done

echo "Database initialized successfully!"

FROM pgvector/pgvector:pg17

ENV POSTGRES_USER=postgres
ENV POSTGRES_PASSWORD=123
ENV POSTGRES_DB=postgres

# sqlite file can be used to inflate the database
COPY --chown=postgres:postgres --chmod=444 data/init.db /opt/sqlite/init.db
# .sql and .sh scripts in /docker-entrypoint-initdb.d are ran automatically
COPY --chown=postgres:postgres --chmod=644 queries/*.sql /docker-entrypoint-initdb.d/
# COPY --chown=postgres:postgres --chmod=744 queries/init_db.sh /opt/postgres/init_db.sh

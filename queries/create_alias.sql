-- Table: public.alias

CREATE TABLE IF NOT EXISTS alias (
  id SERIAL PRIMARY KEY,
  alias TEXT UNIQUE NOT NULL CHECK (alias <> '')
)
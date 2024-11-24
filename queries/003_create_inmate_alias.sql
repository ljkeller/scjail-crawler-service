-- Table: public.inmate_alias

CREATE TABLE IF NOT EXISTS inmate_alias (
  inmate_id INTEGER NOT NULL,
  alias_id INTEGER NOT NULL,
  FOREIGN KEY (inmate_id) REFERENCES inmate(id),
  FOREIGN KEY (alias_id) REFERENCES alias(id),
  PRIMARY KEY (inmate_id, alias_id)
);

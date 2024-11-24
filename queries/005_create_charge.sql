-- Table: public.charge

CREATE TABLE IF NOT EXISTS charge (
  id SERIAL PRIMARY KEY,
  inmate_id INTEGER,
  description TEXT,
  grade TEXT,
  offense_date TEXT,
  FOREIGN KEY (inmate_id) REFERENCES inmate(id)
);

CREATE INDEX idx_inmate_id ON charge(inmate_id);

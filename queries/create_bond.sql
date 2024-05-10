-- Table: public.bond

CREATE TABLE IF NOT EXISTS bond (
  id SERIAL PRIMARY KEY,
  inmate_id INTEGER NOT NULL,
  type TEXT NOT NULL,
  amount_pennies INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (inmate_id) REFERENCES inmate(id) 
)
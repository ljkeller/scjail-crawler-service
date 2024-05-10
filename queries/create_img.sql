-- Table: public.img

CREATE TABLE IF NOT EXISTS img (
  id SERIAL PRIMARY KEY,
  inmate_id INTEGER NOT NULL,
  img BYTEA,
  FOREIGN KEY (inmate_id) REFERENCES inmate(id) 
)
-- Table: public.img

CREATE TABLE IF NOT EXISTS img (
  id SERIAL PRIMARY KEY,
  inmate_id INTEGER NOT NULL,
  img BYTEA,
  FOREIGN KEY (inmate_id) REFERENCES inmate(id) 
);

CREATE INDEX idx_img_inmate_id ON img(inmate_id);

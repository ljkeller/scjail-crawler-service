-- Table: public.inmate

CREATE TABLE IF NOT EXISTS inmate (
  id SERIAL PRIMARY KEY,
  first_name TEXT NOT NULL CHECK (first_name <> ''),
  middle_name TEXT,
  last_name TEXT NOT NULL CHECK (last_name <> ''),
  affix TEXT,
  permanent_id TEXT,
  sex TEXT,
  dob date NOT NULL,
  arresting_agency TEXT,
  booking_date TIMESTAMP WITH TIME ZONE NOT NULL,
  booking_number TEXT,
  height TEXT,
  weight TEXT,
  race TEXT,
  eye_color TEXT,
  img_url TEXT,
  scil_sysid TEXT,
  record_visits INTEGER DEFAULT 0,
  shared INTEGER DEFAULT 0,
  UNIQUE (first_name, last_name, dob, booking_date)
)
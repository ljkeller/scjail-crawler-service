-- Table: public.inmate

CREATE TABLE IF NOT EXISTS public.inmate
(
    id integer NOT NULL DEFAULT nextval('inmate_id_seq'::regclass),
    first_name text COLLATE pg_catalog."default" NOT NULL,
    middle_name text COLLATE pg_catalog."default",
    last_name text COLLATE pg_catalog."default" NOT NULL,
    affix text COLLATE pg_catalog."default",
    permanent_id text COLLATE pg_catalog."default",
    sex text COLLATE pg_catalog."default",
    dob date NOT NULL,
    arresting_agency text COLLATE pg_catalog."default",
    booking_date timestamp with time zone NOT NULL,
    booking_number text COLLATE pg_catalog."default",
    height text COLLATE pg_catalog."default",
    weight text COLLATE pg_catalog."default",
    race text COLLATE pg_catalog."default",
    eye_color text COLLATE pg_catalog."default",
    img_url text COLLATE pg_catalog."default",
    scil_sysid text COLLATE pg_catalog."default",
    record_visits integer DEFAULT 0,
    shared integer DEFAULT 0,
    CONSTRAINT inmate_pkey PRIMARY KEY (id),
    CONSTRAINT inmate_first_name_last_name_dob_booking_date_key UNIQUE (first_name, last_name, dob, booking_date),
    CONSTRAINT inmate_first_name_check CHECK (first_name <> ''::text),
    CONSTRAINT inmate_last_name_check CHECK (last_name <> ''::text)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.inmate
    OWNER to postgres;
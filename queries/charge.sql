-- Table: public.charge

CREATE TABLE IF NOT EXISTS public.charge
(
    id integer NOT NULL DEFAULT nextval('charge_id_seq'::regclass),
    inmate_id integer,
    description text COLLATE pg_catalog."default",
    grade text COLLATE pg_catalog."default",
    offense_date text COLLATE pg_catalog."default",
    CONSTRAINT charge_pkey PRIMARY KEY (id),
    CONSTRAINT charge_inmate_id_fkey FOREIGN KEY (inmate_id)
        REFERENCES public.inmate (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.charge
    OWNER to postgres;
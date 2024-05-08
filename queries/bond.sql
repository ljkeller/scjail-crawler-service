-- Table: public.bond

CREATE TABLE IF NOT EXISTS public.bond
(
    id integer NOT NULL DEFAULT nextval('bond_id_seq'::regclass),
    inmate_id integer NOT NULL,
    type text COLLATE pg_catalog."default" NOT NULL,
    amount_pennies integer NOT NULL DEFAULT 0,
    CONSTRAINT bond_pkey PRIMARY KEY (id),
    CONSTRAINT bond_inmate_id_fkey FOREIGN KEY (inmate_id)
        REFERENCES public.inmate (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.bond
    OWNER to postgres;
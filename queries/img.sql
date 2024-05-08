-- Table: public.img

CREATE TABLE IF NOT EXISTS public.img
(
    id integer NOT NULL DEFAULT nextval('img_id_seq'::regclass),
    inmate_id integer NOT NULL,
    img bytea,
    CONSTRAINT img_pkey PRIMARY KEY (id),
    CONSTRAINT img_inmate_id_fkey FOREIGN KEY (inmate_id)
        REFERENCES public.inmate (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.img
    OWNER to postgres;
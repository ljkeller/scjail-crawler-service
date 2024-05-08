-- Table: public.inmate_alias

CREATE TABLE IF NOT EXISTS public.inmate_alias
(
    inmate_id integer NOT NULL,
    alias_id integer NOT NULL,
    CONSTRAINT inmate_alias_pkey PRIMARY KEY (inmate_id, alias_id),
    CONSTRAINT inmate_alias_alias_id_fkey FOREIGN KEY (alias_id)
        REFERENCES public.alias (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION,
    CONSTRAINT inmate_alias_inmate_id_fkey FOREIGN KEY (inmate_id)
        REFERENCES public.inmate (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.inmate_alias
    OWNER to postgres;
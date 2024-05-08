-- Table: public.alias

CREATE TABLE IF NOT EXISTS public.alias
(
    id integer NOT NULL DEFAULT nextval('alias_id_seq'::regclass),
    alias text COLLATE pg_catalog."default" NOT NULL,
    CONSTRAINT alias_pkey PRIMARY KEY (id),
    CONSTRAINT alias_alias_key UNIQUE (alias),
    CONSTRAINT alias_alias_check CHECK (alias <> ''::text)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.alias
    OWNER to postgres;
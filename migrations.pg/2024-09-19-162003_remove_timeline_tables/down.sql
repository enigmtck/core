CREATE TABLE public.timeline (
    id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    tag jsonb,
    attributed_to character varying NOT NULL,
    ap_id character varying NOT NULL,
    kind public.timeline_type NOT NULL,
    url character varying,
    published character varying,
    replies jsonb,
    in_reply_to character varying,
    content character varying,
    ap_public boolean DEFAULT false NOT NULL,
    summary character varying,
    ap_sensitive boolean,
    atom_uri character varying,
    in_reply_to_atom_uri character varying,
    conversation character varying,
    content_map jsonb,
    attachment jsonb,
    ap_object jsonb,
    metadata jsonb,
    end_time timestamp with time zone,
    one_of jsonb,
    any_of jsonb,
    voters_count integer
);

--ALTER TABLE public.timeline OWNER TO enigmatick;
CREATE SEQUENCE public.timeline_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--ALTER SEQUENCE public.timeline_id_seq OWNER TO enigmatick;
--ALTER SEQUENCE public.timeline_id_seq OWNED BY public.timeline.id;
ALTER TABLE ONLY public.timeline ALTER COLUMN id SET DEFAULT nextval('public.timeline_id_seq'::regclass);

CREATE TABLE public.timeline_to (
    id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    timeline_id integer NOT NULL,
    ap_id character varying NOT NULL COLLATE public.case_insensitive
);

--ALTER TABLE public.timeline_to OWNER TO enigmatick;

CREATE SEQUENCE public.timeline_to_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--ALTER SEQUENCE public.timeline_to_id_seq OWNER TO enigmatick;
--ALTER SEQUENCE public.timeline_to_id_seq OWNED BY public.timeline_to.id;
ALTER TABLE ONLY public.timeline_to ALTER COLUMN id SET DEFAULT nextval('public.timeline_to_id_seq'::regclass);

CREATE TABLE public.timeline_cc (
    id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    timeline_id integer NOT NULL,
    ap_id character varying NOT NULL COLLATE public.case_insensitive
);

--ALTER TABLE public.timeline_cc OWNER TO enigmatick;
CREATE SEQUENCE public.timeline_cc_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--ALTER SEQUENCE public.timeline_cc_id_seq OWNER TO enigmatick;
--ALTER SEQUENCE public.timeline_cc_id_seq OWNED BY public.timeline_cc.id;
ALTER TABLE ONLY public.timeline_cc ALTER COLUMN id SET DEFAULT nextval('public.timeline_cc_id_seq'::regclass);

CREATE TABLE public.timeline_hashtags (
    id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    hashtag character varying NOT NULL COLLATE public.case_insensitive,
    timeline_id integer NOT NULL
);

--ALTER TABLE public.timeline_hashtags OWNER TO enigmatick;

CREATE SEQUENCE public.timeline_hashtags_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

--ALTER SEQUENCE public.timeline_hashtags_id_seq OWNER TO enigmatick;
--ALTER SEQUENCE public.timeline_hashtags_id_seq OWNED BY public.timeline_hashtags.id;
ALTER TABLE ONLY public.timeline_hashtags ALTER COLUMN id SET DEFAULT nextval('public.timeline_hashtags_id_seq'::regclass);


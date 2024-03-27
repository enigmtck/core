CREATE TYPE question_type AS ENUM ('question');

CREATE TABLE remote_questions (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  kind question_type NOT NULL DEFAULT 'question',
  ap_id VARCHAR NOT NULL COLLATE "case_insensitive" UNIQUE,
  ap_to JSONB,
  cc JSONB,
  end_time TIMESTAMPTZ,
  published TIMESTAMPTZ,
  one_of JSONB,
  any_of JSONB,
  content VARCHAR,
  content_map JSONB,
  summary VARCHAR,
  voters_count INTEGER,
  url TEXT,
  conversation TEXT,
  tag JSONB,
  attachment JSONB,
  ap_sensitive BOOLEAN,
  in_reply_to TEXT
);

SELECT diesel_manage_updated_at('remote_questions');

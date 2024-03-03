CREATE TABLE remote_note_hashtags (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  remote_note_id INT NOT NULL,
  CONSTRAINT fk_remote_note_hashtags_remote_note FOREIGN KEY(remote_note_id) REFERENCES remote_notes(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('remote_note_hashtags');

CREATE TABLE remote_actor_hashtags (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  remote_actor_id INT NOT NULL,
  CONSTRAINT fk_remote_actor_hashtags_remote_actor FOREIGN KEY(remote_actor_id) REFERENCES remote_actors(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('remote_actor_hashtags');

CREATE TABLE note_hashtags (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  note_id INT NOT NULL,
  CONSTRAINT fk_note_hashtags_note FOREIGN KEY(note_id) REFERENCES remote_notes(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('note_hashtags');

CREATE TABLE profile_hashtags (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  profile_id INT NOT NULL,
  CONSTRAINT fk_profile_hashtags_profile FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('profile_hashtags');

CREATE TABLE timeline_hashtags (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  timeline_id INT NOT NULL,
  CONSTRAINT fk_timeline_hashtags_note FOREIGN KEY(timeline_id) REFERENCES timeline(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('timeline_hashtags');

CREATE OR REPLACE FUNCTION increment_update_count()
  RETURNS TRIGGER
AS
$$
BEGIN
  NEW.update_count := NEW.update_count + 1;
  RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TABLE hashtag_trend (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  period INT NOT NULL,
  hashtag VARCHAR NOT NULL COLLATE "case_insensitive",
  update_count INT NOT NULL DEFAULT 0
);

CREATE TRIGGER trigger_track_update_count
  BEFORE UPDATE ON hashtag_trend
  FOR EACH ROW
  EXECUTE PROCEDURE increment_update_count();
  
SELECT diesel_manage_updated_at('hashtag_trend');


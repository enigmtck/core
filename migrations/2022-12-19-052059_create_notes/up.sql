CREATE TABLE notes (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR UNIQUE NOT NULL,
  profile_id INT NOT NULL,
  content VARCHAR NOT NULL,
  CONSTRAINT fk_profile_notes FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TABLE note_subjects (
  id SERIAL PRIMARY KEY,
  note_id INT NOT NULL,
  profile_id INT NOT NULL,
  CONSTRAINT fk_note_note_subjects FOREIGN KEY(note_id) REFERENCES notes(id),
  CONSTRAINT fk_profile_note_subjects FOREIGN KEY(profile_id) REFERENCES profiles(id),
  UNIQUE (note_id, profile_id)
);

SELECT diesel_manage_updated_at('notes');

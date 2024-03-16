CREATE TYPE note_type AS ENUM ('note', 'encrypted_note', 'vault_note');

UPDATE notes SET kind = 'note' WHERE kind = 'Note';
UPDATE notes SET kind = 'encrypted_note' WHERE kind = 'EncryptedNote';

ALTER TABLE notes ALTER COLUMN kind TYPE note_type USING (CAST(kind AS note_type));

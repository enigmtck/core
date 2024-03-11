ALTER TABLE notes ALTER COLUMN kind TYPE VARCHAR USING (CAST(kind AS VARCHAR));

UPDATE notes SET kind = 'Note' WHERE kind = 'note';
UPDATE notes SET kind = 'EncryptedNote' WHERE kind = 'encrypted_note';

DROP TYPE note_type;

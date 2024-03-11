ALTER TABLE remote_notes ALTER COLUMN kind TYPE VARCHAR USING (CAST(kind AS VARCHAR));

UPDATE remote_notes SET kind = 'Note' WHERE kind = 'note';
UPDATE remote_notes SET kind = 'EncryptedNote' WHERE kind = 'encrypted_note';
UPDATE remote_notes SET kind = 'Question' WHERE kind = 'question';


ALTER TABLE timeline ALTER COLUMN kind TYPE VARCHAR USING (CAST(kind AS VARCHAR));

UPDATE timeline SET kind = 'Note' WHERE kind = 'note';
UPDATE timeline SET kind = 'EncryptedNote' WHERE kind = 'encrypted_note';
UPDATE timeline SET kind = 'Question' WHERE kind = 'question';

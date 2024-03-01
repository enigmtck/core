ALTER TYPE note_type ADD VALUE IF NOT EXISTS 'question';

UPDATE timeline SET kind = 'note' WHERE kind = 'Note';
UPDATE timeline SET kind = 'encrypted_note' WHERE kind = 'EncryptedNote';
UPDATE timeline SET kind = 'question' WHERE kind = 'Question';

ALTER TABLE timeline ALTER COLUMN kind TYPE note_type USING (CAST(kind AS note_type));

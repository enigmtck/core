UPDATE remote_notes SET kind = 'note' WHERE kind = 'Note';
UPDATE remote_notes SET kind = 'encrypted_note' WHERE kind = 'EncryptedNote';
UPDATE remote_notes SET kind = 'question' WHERE kind = 'Question';

ALTER TABLE remote_notes ALTER COLUMN kind TYPE note_type USING (CAST(kind AS note_type));

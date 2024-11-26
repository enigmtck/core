ALTER TABLE actors ADD COLUMN ek_last_decrypted_activity TIMESTAMPTZ NOT NULL DEFAULT now();

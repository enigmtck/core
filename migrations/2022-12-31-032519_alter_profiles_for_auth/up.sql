ALTER TABLE profiles ADD COLUMN password VARCHAR;
ALTER TABLE profiles ADD COLUMN keystore JSONB NOT NULL;
ALTER TABLE profiles ADD COLUMN client_public_key VARCHAR;

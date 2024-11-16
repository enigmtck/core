ALTER TABLE olm_sessions ADD CONSTRAINT uniq_olm_sessions_uuid UNIQUE (uuid);
ALTER TABLE vault ADD CONSTRAINT uniq_vault_uuid UNIQUE (uuid);

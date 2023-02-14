ALTER TABLE profiles DROP COLUMN salt;
ALTER TABLE profiles DROP COLUMN client_private_key;
ALTER TABLE profiles DROP COLUMN olm_pickled_account;
ALTER TABLE profiles DROP COLUMN olm_identity_key;

DROP TABLE olm_sessions;
DROP TABLE olm_one_time_keys;
DROP TABLE remote_olm_one_time_keys;
DROP TABLE remote_olm_identity_keys;

CREATE COLLATION case_insensitive (provider = icu, locale = 'und-u-ks-level2', DETERMINISTIC = false);
ALTER TABLE profiles ALTER COLUMN username SET DATA TYPE VARCHAR COLLATE "case_insensitive";

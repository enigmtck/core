ALTER TABLE olm_sessions ADD CONSTRAINT uniq_owner_conversation UNIQUE (ap_conversation, owner_as_id);

ALTER TABLE activities DROP CONSTRAINT activities_actor_id_fkey,
	ADD CONSTRAINT activities_actor_id_fkey FOREIGN KEY (actor_id)
	REFERENCES actors(id) ON DELETE CASCADE;

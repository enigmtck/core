ALTER TABLE olm_sessions DROP CONSTRAINT uniq_owner_conversation;

ALTER TABLE activities DROP CONSTRAINT activities_actor_id_fkey,
        ADD CONSTRAINT activities_actor_id_fkey FOREIGN KEY (actor_id)
        REFERENCES actors(id);

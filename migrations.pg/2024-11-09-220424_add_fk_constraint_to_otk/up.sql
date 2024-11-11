ALTER TABLE olm_one_time_keys
      ADD CONSTRAINT fk_actor_olm_one_time_key FOREIGN KEY (profile_id) 
          REFERENCES actors (id) ON DELETE CASCADE;

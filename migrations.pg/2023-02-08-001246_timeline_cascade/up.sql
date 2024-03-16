ALTER TABLE timeline_cc
DROP CONSTRAINT fk_timeline_cc_timeline,
ADD CONSTRAINT fk_timeline_cc_timeline
   FOREIGN KEY (timeline_id)
   REFERENCES timeline(id)
   ON DELETE CASCADE;

ALTER TABLE timeline_to
DROP CONSTRAINT fk_timeline_to_timeline,
ADD CONSTRAINT fk_timeline_to_timeline
   FOREIGN KEY (timeline_id)
   REFERENCES timeline(id)
   ON DELETE CASCADE;

ALTER TABLE remote_announces
DROP CONSTRAINT fk_timeline_remote_announces,
ADD CONSTRAINT fk_timeline_remote_announces
   FOREIGN KEY (timeline_id)
   REFERENCES timeline(id)
   ON DELETE CASCADE;

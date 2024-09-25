CREATE INDEX idx_objects_ek_profile_id ON objects (ek_profile_id);
CREATE INDEX idx_objects_ap_conversation ON objects (ap_conversation);
CREATE INDEX idx_objects_as_cc ON objects USING gin (as_cc);
CREATE INDEX idx_objects_as_bcc ON objects USING gin (as_bcc);
CREATE INDEX idx_objects_as_bto ON objects USING gin (as_bto);
CREATE INDEX idx_objects_as_type_as_published ON objects (as_type, as_published DESC);

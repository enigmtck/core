CREATE TABLE activities_cc (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  activity_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  CONSTRAINT fk_activities_cc_activities FOREIGN KEY(activity_id) REFERENCES activities(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('activities_cc');

CREATE TABLE activities_to (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  activity_id INT NOT NULL,
  ap_id VARCHAR NOT NULL,
  CONSTRAINT fk_activities_to_activities FOREIGN KEY(activity_id) REFERENCES activities(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('activities_to');

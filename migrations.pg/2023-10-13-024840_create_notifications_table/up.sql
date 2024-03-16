CREATE TYPE notification_type AS ENUM ('mention', 'announce', 'unannounce', 'like', 'unlike', 'follow', 'unfollow', 'accept', 'block');

CREATE TABLE notifications (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  uuid VARCHAR NOT NULL,
  kind notification_type NOT NULL,
  profile_id INT NOT NULL,
  activity_id INT NOT NULL
);

SELECT diesel_manage_updated_at('notifications');

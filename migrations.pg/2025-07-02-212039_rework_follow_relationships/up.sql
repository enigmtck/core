DROP TABLE IF EXISTS follows;

CREATE TABLE follows (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    follower_ap_id VARCHAR NOT NULL COLLATE "case_insensitive",          -- AP ID of the initiating actor
    leader_ap_id VARCHAR NOT NULL COLLATE "case_insensitive",            -- AP ID of the target actor
    follow_activity_ap_id VARCHAR COLLATE "case_insensitive",            -- AP ID of original Follow activity
    accept_activity_ap_id VARCHAR COLLATE "case_insensitive",            -- AP ID of Accept activity
    accepted BOOLEAN NOT NULL DEFAULT FALSE,  -- Follow status
    
    -- Unique constraints
    CONSTRAINT uniq_follow_relationship UNIQUE (follower_ap_id, leader_ap_id),
    
    -- Foreign keys to local actors (optional but recommended)
    follower_actor_id INTEGER REFERENCES actors(id) ON DELETE CASCADE,
    leader_actor_id INTEGER REFERENCES actors(id) ON DELETE CASCADE
);

-- Indexes for fast lookups
CREATE INDEX idx_follows_follower ON follows(follower_ap_id);
CREATE INDEX idx_follows_leader ON follows(leader_ap_id);
CREATE INDEX idx_follows_follower_local ON follows(follower_actor_id);
CREATE INDEX idx_follows_leader_local ON follows(leader_actor_id);

-- Keep the auto-update trigger
CREATE TRIGGER set_updated_at
BEFORE UPDATE ON follows
FOR EACH ROW EXECUTE FUNCTION diesel_set_updated_at();

-- Migrate followers (outgoing follows)
INSERT INTO follows (
    created_at, updated_at,
    follower_ap_id, leader_ap_id,
    accepted
)
SELECT 
    created_at, updated_at,
    actor, followed_ap_id,
    'false'
FROM followers
ON CONFLICT (follower_ap_id, leader_ap_id) DO NOTHING;

-- Migrate leaders (incoming follows)
INSERT INTO follows (
    created_at, updated_at,
    follower_ap_id, leader_ap_id,
    accept_activity_ap_id, accepted, follow_activity_ap_id
)
SELECT 
    created_at, updated_at,
    actor, leader_ap_id,
    accept_ap_id, accepted, follow_ap_id
FROM leaders
ON CONFLICT (follower_ap_id, leader_ap_id) DO UPDATE SET
    accepted = EXCLUDED.accepted,
    follow_activity_ap_id = EXCLUDED.follow_activity_ap_id,
    accept_activity_ap_id = EXCLUDED.accept_activity_ap_id;

UPDATE follows
SET follow_activity_ap_id = sub.ap_id
FROM (
    SELECT DISTINCT ON (f.follower_ap_id, f.leader_ap_id)
        f.follower_ap_id,
        f.leader_ap_id,
        a.ap_id,
        a.created_at
    FROM follows f
    JOIN activities a
      ON a.kind = 'follow'
     AND a.actor = f.follower_ap_id
     AND a.target_ap_id = f.leader_ap_id
    ORDER BY f.follower_ap_id, f.leader_ap_id, a.created_at DESC
) AS sub
WHERE follows.follower_ap_id = sub.follower_ap_id
  AND follows.leader_ap_id = sub.leader_ap_id;

UPDATE follows
SET
    accept_activity_ap_id = sub.ap_id,
    accepted = TRUE
FROM (
    SELECT DISTINCT ON (f.follower_ap_id, f.leader_ap_id)
        f.follower_ap_id,
        f.leader_ap_id,
        a.ap_id,
        a.created_at
    FROM follows f
    JOIN activities a
      ON a.kind = 'accept'
     AND a.actor = f.leader_ap_id
     AND a.target_ap_id = f.follow_activity_ap_id
    ORDER BY f.follower_ap_id, f.leader_ap_id, a.created_at DESC
) AS sub
WHERE follows.follower_ap_id = sub.follower_ap_id
  AND follows.leader_ap_id = sub.leader_ap_id;

-- Update follower_actor_id where it is null
UPDATE follows
SET follower_actor_id = a.id
FROM actors a
WHERE follows.follower_actor_id IS NULL
  AND follows.follower_ap_id = a.as_id;

-- Update leader_actor_id where it is null
UPDATE follows
SET leader_actor_id = a.id
FROM actors a
WHERE follows.leader_actor_id IS NULL
  AND follows.leader_ap_id = a.as_id;

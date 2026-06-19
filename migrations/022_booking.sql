-- Booking integration (lot 2) per-site configuration and unit selection.
-- NOTE the migration runner (doxyde-db/src/init.rs) splits statements on the
-- semicolon character and skips comment-only fragments, so every statement must
-- be standalone (no embedded semicolons, including inside comments) and
-- idempotent (re-runnable across many site DBs).

-- Per-site booking service connection (one singleton row). service_url +
-- service_secret point at the sejours-api microservice for this site.
CREATE TABLE IF NOT EXISTS booking_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    service_url TEXT NOT NULL DEFAULT '',
    service_secret TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO booking_config (id, service_url, service_secret)
SELECT 1, '', ''
WHERE NOT EXISTS (SELECT 1 FROM booking_config WHERE id = 1);

-- Units this site offers, grouped by role. role is 'primary' (shown first) or
-- 'secondary' (offered as the sister-house alternative). position orders within
-- a role. listing_id is the Hostaway listing id exposed by sejours-api.
CREATE TABLE IF NOT EXISTS booking_listing (
    listing_id INTEGER PRIMARY KEY,
    role TEXT NOT NULL DEFAULT 'primary',
    position INTEGER NOT NULL DEFAULT 0
);

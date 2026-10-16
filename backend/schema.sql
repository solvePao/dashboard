-- Initial schema for the dashboard backend (Cloudflare D1 / SQLite).
-- Local:  wrangler d1 execute dashboard --local --file=./schema.sql
-- Remote: wrangler d1 execute dashboard --remote --file=./schema.sql

CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  google_sub TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL,
  name TEXT,
  picture_url TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  last_login_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

-- Pairing code index (pointer only, 2 min expiration). Pairing state
-- itself lives in the PairingObject Durable Object; this table only
-- translates the 6-char code typed on the web into a session_id.
CREATE TABLE IF NOT EXISTS pairing_codes (
  session_id TEXT PRIMARY KEY,
  short_code TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pairing_codes_short_code ON pairing_codes (short_code);

-- Long-lived device tokens for mobile clients (6 months validity)
CREATE TABLE IF NOT EXISTS device_tokens (
  token TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id),
  device_name TEXT NOT NULL DEFAULT 'Mobile Device',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  expires_at TEXT NOT NULL,
  last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_device_tokens_user ON device_tokens (user_id);


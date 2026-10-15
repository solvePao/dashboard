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

-- Device pairing sessions (temporary handshake, 15 min expiration)
CREATE TABLE IF NOT EXISTS device_sessions (
  session_id TEXT PRIMARY KEY,
  short_code TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'authorized', 'expired'
  user_id INTEGER REFERENCES users(id),
  device_token TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  expires_at TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_device_sessions_short_code ON device_sessions (short_code);
CREATE INDEX IF NOT EXISTS idx_device_sessions_status ON device_sessions (status);

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

CREATE INDEX IF NOT EXISTS idx_pairing_codes_lookup
ON pairing_codes (lower(short_code), created_at DESC, expires_at);

CREATE INDEX IF NOT EXISTS idx_walkie_rate_limits_window
ON walkie_rate_limits (window);

CREATE INDEX IF NOT EXISTS idx_walkie_device_members_token
ON walkie_device_members (token, group_id);

CREATE INDEX IF NOT EXISTS idx_walkie_groups_owner_created
ON walkie_groups (owner_id, created_at);

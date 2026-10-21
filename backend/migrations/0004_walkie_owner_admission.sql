-- All phones must be admitted by an owner using a code generated on that phone.
-- Preserve groups, ownership, device authorization, and widget enablement.
DELETE FROM walkie_device_members;
DELETE FROM walkie_assignments;
UPDATE device_widgets SET config_json='{}' WHERE widget_key='walkie_talkie';
DROP TABLE walkie_invites;
CREATE TABLE walkie_device_codes (
  token TEXT PRIMARY KEY REFERENCES walkie_devices(token) ON DELETE CASCADE,
  code TEXT NOT NULL UNIQUE,
  expires_at INTEGER NOT NULL,
  consumed_at INTEGER,
  redeemed_group TEXT
);

-- Membership belongs to an authorized device; ownership remains with its account.
CREATE TABLE walkie_devices (
  token TEXT PRIMARY KEY REFERENCES device_tokens(token) ON DELETE CASCADE,
  device_id TEXT NOT NULL UNIQUE DEFAULT (lower(hex(randomblob(16))))
);
INSERT INTO walkie_devices(token) SELECT token FROM device_tokens;
CREATE TABLE walkie_device_members (
  group_id TEXT NOT NULL REFERENCES walkie_groups(id) ON DELETE CASCADE,
  token TEXT NOT NULL REFERENCES walkie_devices(token) ON DELETE CASCADE,
  PRIMARY KEY(group_id,token)
);
INSERT INTO walkie_device_members(group_id,token) SELECT group_id,token FROM walkie_assignments;
CREATE TABLE walkie_active_backup AS SELECT * FROM walkie_assignments;
DROP TRIGGER walkie_assignment_cleared;
DROP TABLE walkie_assignments;
DROP TABLE walkie_members;
CREATE TABLE walkie_assignments (
  token TEXT PRIMARY KEY,
  group_id TEXT NOT NULL,
  user_id INTEGER NOT NULL REFERENCES users(id),
  FOREIGN KEY(group_id,token) REFERENCES walkie_device_members(group_id,token) ON DELETE CASCADE
);
INSERT INTO walkie_assignments SELECT token,group_id,user_id FROM walkie_active_backup;
DROP TABLE walkie_active_backup;
CREATE TRIGGER walkie_device_limit BEFORE INSERT ON walkie_device_members
WHEN NOT EXISTS(SELECT 1 FROM walkie_device_members WHERE group_id=NEW.group_id AND token=NEW.token)
 AND (SELECT count(*) FROM walkie_device_members WHERE group_id=NEW.group_id)>=5
BEGIN SELECT RAISE(ABORT,'walkie_group_full'); END;
CREATE TRIGGER walkie_assignment_cleared AFTER DELETE ON walkie_assignments
BEGIN
 UPDATE device_widgets SET config_json='{}' WHERE token=OLD.token AND widget_key='walkie_talkie';
END;
-- Retain a redemption receipt until expiry so retrying the same device is idempotent.
ALTER TABLE walkie_invites ADD COLUMN consumed_by TEXT;

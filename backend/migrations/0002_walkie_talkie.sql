CREATE TABLE walkie_groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 60),
  owner_id INTEGER NOT NULL REFERENCES users(id),
  created_at INTEGER NOT NULL
);
CREATE TABLE walkie_members (
  group_id TEXT NOT NULL REFERENCES walkie_groups(id) ON DELETE CASCADE,
  user_id INTEGER NOT NULL REFERENCES users(id),
  PRIMARY KEY(group_id, user_id)
);
CREATE INDEX walkie_members_user ON walkie_members(user_id);
CREATE TRIGGER walkie_member_limit BEFORE INSERT ON walkie_members
WHEN NOT EXISTS(SELECT 1 FROM walkie_members WHERE group_id=NEW.group_id AND user_id=NEW.user_id)
 AND (SELECT count(*) FROM walkie_members WHERE group_id=NEW.group_id) >= 5
BEGIN SELECT RAISE(ABORT, 'walkie_group_full'); END;
CREATE TABLE walkie_invites (
  group_id TEXT PRIMARY KEY REFERENCES walkie_groups(id) ON DELETE CASCADE,
  code TEXT NOT NULL UNIQUE,
  expires_at INTEGER NOT NULL
);
CREATE TABLE walkie_assignments (
  token TEXT PRIMARY KEY REFERENCES device_tokens(token) ON DELETE CASCADE,
  group_id TEXT NOT NULL,
  user_id INTEGER NOT NULL,
  UNIQUE(group_id, user_id),
  FOREIGN KEY(group_id,user_id) REFERENCES walkie_members(group_id,user_id) ON DELETE CASCADE
);
CREATE TABLE walkie_rate_limits (
  bucket TEXT PRIMARY KEY,
  window INTEGER NOT NULL,
  attempts INTEGER NOT NULL
);

-- All assignment removal paths clear the device-facing selection atomically.
CREATE TRIGGER walkie_assignment_cleared AFTER DELETE ON walkie_assignments
BEGIN
  UPDATE device_widgets SET config_json='{}'
  WHERE token=OLD.token AND widget_key='walkie_talkie';
END;

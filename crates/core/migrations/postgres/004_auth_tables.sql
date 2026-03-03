-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Refresh tokens table
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Add user_id column to existing tables
ALTER TABLE collections ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE environments ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE history_entries ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE folders ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE saved_requests ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE variables ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE ws_sessions ADD COLUMN IF NOT EXISTS user_id TEXT;

-- Indexes for user_id scoping
CREATE INDEX IF NOT EXISTS idx_collections_user_id ON collections(user_id);
CREATE INDEX IF NOT EXISTS idx_environments_user_id ON environments(user_id);
CREATE INDEX IF NOT EXISTS idx_history_entries_user_id ON history_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_folders_user_id ON folders(user_id);
CREATE INDEX IF NOT EXISTS idx_saved_requests_user_id ON saved_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_variables_user_id ON variables(user_id);
CREATE INDEX IF NOT EXISTS idx_ws_sessions_user_id ON ws_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);

-- Collections
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    auth TEXT NOT NULL DEFAULT '{"type":"none"}',
    pre_request_script TEXT,
    test_script TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Folders
CREATE TABLE folders (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    parent_folder_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Saved Requests
CREATE TABLE saved_requests (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    folder_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    headers TEXT NOT NULL DEFAULT '{}',
    body TEXT,
    params TEXT NOT NULL DEFAULT '[]',
    auth TEXT NOT NULL DEFAULT '{"type":"none"}',
    pre_request_script TEXT,
    test_script TEXT,
    request_type TEXT NOT NULL DEFAULT 'http',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- History
CREATE TABLE history_entries (
    id TEXT PRIMARY KEY,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    request_headers TEXT NOT NULL,
    request_body TEXT,
    response_status INTEGER NOT NULL,
    response_headers TEXT NOT NULL,
    response_body TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    timestamp TEXT NOT NULL
);

-- Environments
CREATE TABLE environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Variables
CREATE TABLE variables (
    id TEXT PRIMARY KEY,
    environment_id TEXT REFERENCES environments(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    is_secret INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_variables_environment ON variables(environment_id);
CREATE UNIQUE INDEX idx_variables_env_key ON variables(environment_id, key);

-- WebSocket Sessions
CREATE TABLE ws_sessions (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    connected_at TEXT NOT NULL,
    disconnected_at TEXT,
    duration_ms INTEGER,
    message_count INTEGER NOT NULL DEFAULT 0
);

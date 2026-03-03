CREATE TABLE IF NOT EXISTS server_connections (
    id TEXT PRIMARY KEY,
    server_url TEXT NOT NULL,
    display_name TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    user_email TEXT,
    user_display_name TEXT,
    user_server_id TEXT,
    server_mode TEXT NOT NULL DEFAULT 'personal',
    status TEXT NOT NULL DEFAULT 'disconnected',
    created_at TEXT NOT NULL,
    last_used_at TEXT
);

# apihop

A fast, open-source API client for REST, GraphQL, and WebSocket — runs as a desktop app (Tauri) or self-hosted web server (Axum).

## Features

**HTTP Client**
- Full request builder with GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- Query parameters, headers, and body editors with key-value tables
- Multiple body types: JSON, form-data, URL-encoded, raw, binary
- Response viewer with syntax highlighting, timing metrics, and size info
- Request history with automatic recording

**Authentication**
- Basic Auth, Bearer Token, API Key (header/query)
- OAuth2 Client Credentials flow
- Collection-level auth inheritance

**GraphQL**
- Dedicated GraphQL IDE with schema introspection
- Visual query builder with field selection
- Schema explorer for types and fields
- Variables and operation name support

**WebSocket**
- Real-time connection management
- Send/receive messages with direction tracking
- Binary and text message support
- Connection status monitoring and session metrics

**Scripting & Automation**
- Pre-request scripts to modify requests before sending
- Test scripts with assertions for response validation
- Postman-compatible `pm` API (`pm.variables`, `pm.request`, `pm.response`, `pm.test()`)
- JavaScript execution via QuickJS engine

**Data Extraction**
- Extract values from responses using JSONPath, headers, or status codes
- Automatically set variables from response data for request chaining

**Collections & Organization**
- Collections with nested folder hierarchy
- Import: Postman v2.1, OpenAPI 3.x, cURL
- Export: apihop JSON, Postman v2.1, cURL

**Environments & Variables**
- Global and environment-scoped variables
- `{{variable}}` interpolation in URLs, headers, body, and auth fields
- Built-in dynamic variables: `$timestamp`, `$randomUUID`, `$randomEmail`, and more
- Secret variables with encrypted storage
- Variable autocomplete in editors

**Multi-User (Server Mode)**
- User accounts with JWT authentication
- Workspaces with role-based access (Owner, Editor, Viewer)
- Invite users via email

## Architecture

```
crates/
├── core/       # Shared business logic (database, pipeline, scripting)
├── server/     # Axum web server — serves UI + REST API
└── desktop/    # Tauri v2 desktop app

ui/             # Vue 3 + TypeScript frontend (shared by both modes)
```

- **`core`** — All business logic lives here. Pure async Rust, no transport awareness.
- **`server`** and **`desktop`** are thin wrappers that expose core functions via HTTP routes or Tauri commands.
- **`ui`** — Single frontend shared by both modes. An adaptive API client (`src/api/client.ts`) detects Tauri vs browser and routes calls accordingly.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | Vue 3, TypeScript, Vite, Pinia, CodeMirror 6 |
| Desktop | Tauri v2, system keyring for secrets |
| Server | Axum, Tower |
| Core | Tokio, Reqwest, SQLx, QuickJS (rquickjs) |
| Database | SQLite (default) or PostgreSQL |
| Package manager | Bun (frontend), Cargo (Rust) |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2024)
- [Bun](https://bun.sh/)
- For desktop mode: [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)

### Web Mode

```bash
# Install frontend dependencies and build
cd ui && bun install && bun run build && cd ..

# Run the server (serves on http://localhost:3000)
cargo run -p apihop-server
```

### Desktop Mode

```bash
cd ui && bun install && cd ..

# Run desktop app with hot-reload
cd crates/desktop && cargo tauri dev
```

### Development

```bash
# Frontend dev server with hot-reload (http://localhost:5173)
cd ui && bun run dev

# Build all Rust crates
cargo build --workspace
```

### Configuration (Server Mode)

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `APIHOP_HOST` | `0.0.0.0` | Bind address |
| `APIHOP_PORT` | `3000` | Port |
| `APIHOP_DATABASE_URL` | `apihop.db` | SQLite path or PostgreSQL connection string |
| `APIHOP_SECRET_KEY` | (auto-generated) | 32-byte hex key for encrypting secrets |
| `APIHOP_MODE` | `personal` | `personal` or `organization` |
| `APIHOP_JWT_SECRET` | — | JWT signing secret (required in organization mode) |
| `APIHOP_REGISTRATION_ENABLED` | `true` | Allow new user registration |

## License

MIT

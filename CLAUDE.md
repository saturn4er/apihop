# apihop

Postman alternative with dual runtime modes: desktop (Tauri v2) and web (Axum server).

## Architecture

- `crates/core/` - shared business logic library (`apihop-core`). Pure async functions, no transport awareness.
- `crates/server/` - Axum web server (`apihop-server`). Serves the Vue frontend from `ui/dist` and exposes `/api/` routes.
- `crates/desktop/` - Tauri v2 desktop app (`apihop-desktop`). Wraps core functions as Tauri commands.
- `ui/` - Vue 3 + TypeScript frontend shared by both modes. Uses adaptive API client (`src/api/client.ts`) that detects Tauri vs browser and routes calls accordingly.

## Build & Run

```bash
# Install frontend dependencies
cd ui && bun install

# Build frontend
cd ui && bun run build

# Build all Rust crates
cargo build --workspace

# Run web mode (serves on http://localhost:3000)
cargo run -p apihop-server

# Run desktop mode
cd crates/desktop && cargo tauri dev

# Frontend dev server only
cd ui && bun run dev
```

## Code Conventions

- Rust edition 2024
- All business logic goes in `apihop-core`, never in server or desktop crates directly
- Server and desktop crates are thin wrappers around core
- Frontend API calls go through `ui/src/api/client.ts` adapter, never call fetch/invoke directly from components
- Use `bun` as the JS package manager

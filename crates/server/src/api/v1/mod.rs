mod auth;
mod collections;
mod connections;
mod environments;
mod folders;
mod graphql;
mod history;
mod import_export;
mod requests;
mod send;
mod server_info;
mod websocket;
mod workspaces;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(send::router())
        .merge(collections::router())
        .merge(folders::router())
        .merge(requests::router())
        .merge(history::router())
        .merge(environments::router())
        .merge(import_export::router())
        .merge(websocket::router())
        .merge(graphql::router())
        .merge(server_info::router())
        .merge(auth::router())
        .merge(connections::router())
        .merge(workspaces::router())
}

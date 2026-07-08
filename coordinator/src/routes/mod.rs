use axum::{
    routing::{delete, get, options, patch, post},
    Router,
};

use crate::state::AppState;

pub mod auth;
pub mod cameras;
pub mod dashboard_ws;
pub mod events;
pub mod node_ws;
pub mod nodes;
pub mod playback;
pub mod retention;
pub mod users;
pub mod whep;

pub fn create_router(state: AppState) -> Router {
    let api = Router::new()
        // auth
        .route("/auth/login",   post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout",  post(auth::logout))
        // users
        .route("/users",     get(users::list_users).post(users::create_user))
        .route("/users/:id", get(users::get_user).patch(users::update_user).delete(users::delete_user))
        // nodes
        .route("/nodes/register",   post(nodes::self_register))
        .route("/nodes",            get(nodes::list_nodes).post(nodes::create_node))
        .route("/nodes/:id",          get(nodes::get_node).delete(nodes::delete_node))
        .route("/nodes/:id/status",   patch(nodes::update_node_status))
        .route("/nodes/:id/capacity", get(nodes::get_node_capacity))
        // node websocket
        .route("/ws/node", get(node_ws::node_ws))
        // cameras
        .route("/cameras",     get(cameras::list_cameras).post(cameras::create_camera))
        .route("/cameras/:id", get(cameras::get_camera).patch(cameras::update_camera).delete(cameras::delete_camera))
        // WHEP proxy for WebRTC live view
        .route("/cameras/:id/whep", post(whep::whep_proxy).options(whep::whep_preflight))
        // retention policies
        .route("/retention-policies",     get(retention::list_retention_policies).post(retention::create_retention_policy))
        .route("/retention-policies/:id", get(retention::get_retention_policy).patch(retention::update_retention_policy).delete(retention::delete_retention_policy))
        // events — motion markers
        .route("/cameras/:id/events",               get(events::list_camera_events))
        // playback — timeline + segment serving + clip export
        .route("/cameras/:id/recordings",           get(playback::list_recordings))
        .route("/cameras/:id/segments/:recording_id", get(playback::get_segment))
        .route("/cameras/:id/export",               get(playback::export_clip))
        // dashboard browser WebSocket
        .route("/ws/dashboard", get(dashboard_ws::dashboard_ws));

    Router::new()
        .nest("/api/v1", api)
        .with_state(state)
}

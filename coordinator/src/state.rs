use common::{auth::JwtConfig, ws::WsMessage};
use serde::Serialize;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

pub type NodeTx      = mpsc::UnboundedSender<String>;
pub type DashboardTx = mpsc::UnboundedSender<String>;

#[derive(Clone)]
pub struct AppState {
    pub db:         PgPool,
    pub jwt:        Arc<JwtConfig>,
    pub nodes:      Arc<RwLock<HashMap<Uuid, NodeTx>>>,
    pub dashboards: Arc<RwLock<HashMap<Uuid, DashboardTx>>>,
}

impl AppState {
    pub fn new(db: PgPool, jwt_secret: &[u8]) -> Self {
        Self {
            db,
            jwt: Arc::new(JwtConfig::from_secret(jwt_secret)),
            nodes:      Arc::new(RwLock::new(HashMap::new())),
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Send a WS message to a connected node, silently drops if not connected.
    pub async fn push_to_node(&self, node_id: Uuid, msg: WsMessage) {
        if let Ok(text) = serde_json::to_string(&msg) {
            let nodes = self.nodes.read().await;
            if let Some(tx) = nodes.get(&node_id) {
                let _ = tx.send(text);
            }
        }
    }

    /// Broadcast a dashboard message to all connected browser clients.
    pub async fn broadcast_dashboard<T: Serialize>(&self, msg: &T) {
        if let Ok(text) = serde_json::to_string(msg) {
            let dashboards = self.dashboards.read().await;
            for tx in dashboards.values() {
                let _ = tx.send(text.clone());
            }
        }
    }
}

use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node_id: Uuid,
    pub api_key: String,
}

impl NodeIdentity {
    /// Load identity with the following priority:
    ///
    /// 1. `NODE_ID` + `NODE_API_KEY` environment variables — used when the node
    ///    is pre-provisioned by an admin via `POST /api/v1/nodes`.  The env vars
    ///    are the canonical source of truth; no file is written or read.
    /// 2. `{data_dir}/state.json` — written on first successful self-registration.
    pub fn load(data_dir: &Path) -> Option<Self> {
        // Priority 1: pre-provisioned identity from env vars
        if let (Ok(raw_id), Ok(api_key)) = (
            std::env::var("NODE_ID"),
            std::env::var("NODE_API_KEY"),
        ) {
            if let Ok(node_id) = Uuid::parse_str(&raw_id) {
                tracing::info!("using pre-provisioned identity from env (node_id={node_id})");
                return Some(Self { node_id, api_key });
            }
        }

        // Priority 2: persisted identity from previous self-registration
        let bytes = std::fs::read(data_dir.join("state.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(data_dir)?;
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(data_dir.join("state.json"), bytes)?;
        Ok(())
    }
}

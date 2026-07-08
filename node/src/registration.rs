use anyhow::Context;
use common::ws::HardwareProfile;

use crate::{config::NodeConfig, identity::NodeIdentity};

#[derive(serde::Deserialize)]
struct RegisterResponse {
    node: NodeRow,
    api_key: String,
}

#[derive(serde::Deserialize)]
struct NodeRow {
    id: uuid::Uuid,
}

pub async fn register(config: &NodeConfig, hardware: HardwareProfile) -> anyhow::Result<NodeIdentity> {
    let name = std::env::var("NODE_NAME").unwrap_or_else(|_| {
        hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unnamed-node".into())
    });

    let url = format!("{}/api/v1/nodes/register", config.coordinator_url);
    let resp: RegisterResponse = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({ "name": name, "hardware": hardware }))
        .send()
        .await
        .context("register request")?
        .error_for_status()
        .context("register response")?
        .json()
        .await
        .context("register decode")?;

    Ok(NodeIdentity { node_id: resp.node.id, api_key: resp.api_key })
}

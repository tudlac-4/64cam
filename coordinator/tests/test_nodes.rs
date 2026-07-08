mod helpers;
use helpers::TestApp;

#[tokio::test]
async fn create_node_returns_api_key() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let resp = app
        .client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": "test-node"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["api_key"].is_string());
    assert_eq!(body["node"]["status"], "pending");
}

#[tokio::test]
async fn approve_node_changes_status() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let node: serde_json::Value = app
        .client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": "to-approve"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let id = node["node"]["id"].as_str().unwrap();

    let resp = app
        .client
        .patch(app.url(&format!("/api/v1/nodes/{id}/status")))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"status": "approved"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["status"], "approved");
}

#[tokio::test]
async fn list_nodes_returns_200() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let resp = app
        .client
        .get(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn delete_node_returns_204() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let node: serde_json::Value = app
        .client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": "to-delete"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let id = node["node"]["id"].as_str().unwrap();

    let resp = app
        .client
        .delete(app.url(&format!("/api/v1/nodes/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .client
        .get(app.url(&format!("/api/v1/nodes/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

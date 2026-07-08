mod helpers;
use helpers::TestApp;

async fn seed_node(app: &TestApp, token: &str) -> serde_json::Value {
    app.client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": format!("node-{}", uuid::Uuid::new_v4())}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()
}

#[tokio::test]
async fn create_and_list_cameras() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let node = seed_node(&app, &token).await;
    let node_id = node["node"]["id"].as_str().unwrap();

    let resp = app
        .client
        .post(app.url("/api/v1/cameras"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "node_id": node_id,
            "name": "Front Door",
            "rtsp_url": "rtsp://192.168.1.1:554/stream",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let cam: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(cam["name"], "Front Door");

    let resp = app
        .client
        .get(app.url(&format!("/api/v1/cameras?node_id={node_id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn update_camera_enabled_flag() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let node = seed_node(&app, &token).await;
    let node_id = node["node"]["id"].as_str().unwrap();

    let cam: serde_json::Value = app
        .client
        .post(app.url("/api/v1/cameras"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"node_id": node_id, "name": "Cam", "rtsp_url": "rtsp://x/s"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let id = cam["id"].as_str().unwrap();

    let resp = app
        .client
        .patch(app.url(&format!("/api/v1/cameras/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"enabled": false}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let updated: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(updated["enabled"], false);
}

#[tokio::test]
async fn delete_camera_returns_204() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let node = seed_node(&app, &token).await;
    let node_id = node["node"]["id"].as_str().unwrap();

    let cam: serde_json::Value = app
        .client
        .post(app.url("/api/v1/cameras"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"node_id": node_id, "name": "Tmp", "rtsp_url": "rtsp://x/s"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let id = cam["id"].as_str().unwrap();

    let resp = app
        .client
        .delete(app.url(&format!("/api/v1/cameras/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}

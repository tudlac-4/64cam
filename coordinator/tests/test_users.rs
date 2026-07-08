mod helpers;
use helpers::TestApp;

#[tokio::test]
async fn list_users_requires_auth() {
    let app = TestApp::spawn().await;
    let resp = app.client.get(app.url("/api/v1/users")).send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn list_users_returns_200_for_admin() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let resp = app
        .client
        .get(app.url("/api/v1/users"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn create_and_get_user() {
    let app = TestApp::spawn().await;
    let (email, password, role_id) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let new_email = format!("viewer-{}@test.local", uuid::Uuid::new_v4());
    let viewer_role_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM roles WHERE name = 'viewer'")
            .fetch_one(&app.db)
            .await
            .unwrap();

    let resp = app
        .client
        .post(app.url("/api/v1/users"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "email": new_email,
            "password": "ViewerPass1!",
            "role_id": viewer_role_id,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    // Fetch by id
    let resp = app
        .client
        .get(app.url(&format!("/api/v1/users/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let _ = role_id; // suppress unused warning
}

#[tokio::test]
async fn delete_user_returns_204() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let viewer_role_id: uuid::Uuid =
        sqlx::query_scalar!("SELECT id FROM roles WHERE name = 'viewer'")
            .fetch_one(&app.db)
            .await
            .unwrap();

    let resp = app
        .client
        .post(app.url("/api/v1/users"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "email": format!("del-{}@test.local", uuid::Uuid::new_v4()),
            "password": "Temp1234!",
            "role_id": viewer_role_id,
        }))
        .send()
        .await
        .unwrap();

    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let resp = app
        .client
        .delete(app.url(&format!("/api/v1/users/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .client
        .get(app.url(&format!("/api/v1/users/{id}")))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

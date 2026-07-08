mod helpers;
use helpers::TestApp;

#[tokio::test]
async fn login_success_returns_tokens() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;

    let resp = app
        .client
        .post(app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({"email": email, "password": password}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert_eq!(body["token_type"], "Bearer");
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let app = TestApp::spawn().await;
    let (email, _, _) = app.seed_admin().await;

    let resp = app
        .client
        .post(app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({"email": email, "password": "wrongpassword"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn login_unknown_email_returns_401() {
    let app = TestApp::spawn().await;

    let resp = app
        .client
        .post(app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({"email": "nobody@test.local", "password": "anything"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn refresh_returns_new_access_token() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;

    let login: serde_json::Value = app
        .client
        .post(app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({"email": email, "password": password}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let refresh_token = login["refresh_token"].as_str().unwrap();

    let resp = app
        .client
        .post(app.url("/api/v1/auth/refresh"))
        .json(&serde_json::json!({"refresh_token": refresh_token}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["access_token"].is_string());
}

#[tokio::test]
async fn refresh_invalidates_old_token() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;

    let login: serde_json::Value = app
        .client
        .post(app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({"email": email, "password": password}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let refresh_token = login["refresh_token"].as_str().unwrap().to_owned();

    // First use succeeds and rotates the token
    app.client
        .post(app.url("/api/v1/auth/refresh"))
        .json(&serde_json::json!({"refresh_token": refresh_token}))
        .send()
        .await
        .unwrap();

    // Second use of the same token must fail
    let resp = app
        .client
        .post(app.url("/api/v1/auth/refresh"))
        .json(&serde_json::json!({"refresh_token": refresh_token}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn logout_requires_auth() {
    let app = TestApp::spawn().await;
    let resp = app
        .client
        .post(app.url("/api/v1/auth/logout"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn logout_succeeds_with_valid_token() {
    let app = TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    let resp = app
        .client
        .post(app.url("/api/v1/auth/logout"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);
}

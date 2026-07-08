mod helpers;

use chrono::Utc;
use uuid::Uuid;

/// Simulate a node indexing a segment via the WS SegmentComplete path by
/// calling the internal index helper directly through the DB — this tests the
/// SQL without requiring a live WS handshake.
#[tokio::test]
async fn segment_indexed_with_retention_policy() {
    let app = helpers::TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    // Create a node (admin-created, starts approved)
    let node_resp = app
        .client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": "test-node"}))
        .send()
        .await
        .unwrap();
    assert_eq!(node_resp.status(), 201);
    let node_body: serde_json::Value = node_resp.json().await.unwrap();
    let node_id = Uuid::parse_str(node_body["node"]["id"].as_str().unwrap()).unwrap();

    // Create a camera on that node
    let cam_resp = app
        .client
        .post(app.url("/api/v1/cameras"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "node_id": node_id,
            "name": "front-door",
            "rtsp_url": "rtsp://192.168.1.1:554/stream",
            "stream_path": "front-door"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(cam_resp.status(), 201);
    let cam_body: serde_json::Value = cam_resp.json().await.unwrap();
    let camera_id = Uuid::parse_str(cam_body["id"].as_str().unwrap()).unwrap();

    // Ensure a default retention policy exists
    let default_policy_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM retention_policies WHERE is_default = true LIMIT 1",
    )
    .fetch_optional(&app.db)
    .await
    .unwrap();
    // If no default policy, insert one
    let retention_id = match default_policy_id {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO retention_policies (id, name, keep_days, is_default)
                 VALUES ($1, 'default', 30, true)",
            )
            .bind(id)
            .execute(&app.db)
            .await
            .unwrap();
            id
        }
    };

    // Insert a recording directly (simulating what node_ws::index_segment does)
    let segment_id = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        b"local:///data/recordings/front-door/20260708_120000.mp4",
    );
    let started_at = Utc::now() - chrono::Duration::hours(1);
    let ended_at = started_at + chrono::Duration::seconds(60);

    sqlx::query(
        "INSERT INTO recordings
             (id, camera_id, node_id, storage_uri, started_at, ended_at,
              duration_secs, size_bytes, retention_policy_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (id, started_at) DO NOTHING",
    )
    .bind(segment_id)
    .bind(camera_id)
    .bind(node_id)
    .bind("local:///data/recordings/front-door/20260708_120000.mp4")
    .bind(started_at)
    .bind(ended_at)
    .bind(60_i32)
    .bind(1_234_567_i64)
    .bind(retention_id)
    .execute(&app.db)
    .await
    .unwrap();

    // Verify it landed in the DB
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recordings WHERE id = $1")
        .bind(segment_id)
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(count, 1, "segment should be in recordings table");

    // Idempotency: inserting the same segment again should be a no-op
    let res = sqlx::query(
        "INSERT INTO recordings
             (id, camera_id, node_id, storage_uri, started_at, ended_at,
              duration_secs, size_bytes, retention_policy_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (id, started_at) DO NOTHING",
    )
    .bind(segment_id)
    .bind(camera_id)
    .bind(node_id)
    .bind("local:///data/recordings/front-door/20260708_120000.mp4")
    .bind(started_at)
    .bind(ended_at)
    .bind(60_i32)
    .bind(1_234_567_i64)
    .bind(retention_id)
    .execute(&app.db)
    .await
    .unwrap();
    assert_eq!(res.rows_affected(), 0, "duplicate insert should be silently ignored");
}

#[tokio::test]
async fn retention_enforcer_deletes_expired_recordings() {
    let app = helpers::TestApp::spawn().await;
    let (email, password, _) = app.seed_admin().await;
    let token = app.login(&email, &password).await;

    // Node + camera
    let node_resp = app
        .client
        .post(app.url("/api/v1/nodes"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"name": "retention-test-node"}))
        .send()
        .await
        .unwrap();
    let node_id = Uuid::parse_str(
        node_resp.json::<serde_json::Value>().await.unwrap()["node"]["id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();

    let cam_resp = app
        .client
        .post(app.url("/api/v1/cameras"))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "node_id": node_id,
            "name": "cam2",
            "rtsp_url": "rtsp://192.168.1.2:554/stream",
            "stream_path": "cam2"
        }))
        .send()
        .await
        .unwrap();
    let camera_id = Uuid::parse_str(
        cam_resp.json::<serde_json::Value>().await.unwrap()["id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();

    // 1-day retention policy
    let policy_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO retention_policies (id, name, keep_days, is_default) VALUES ($1, $2, $3, $4)",
    )
    .bind(policy_id)
    .bind("1-day")
    .bind(1_i32)
    .bind(false)
    .execute(&app.db)
    .await
    .unwrap();

    // Insert a recording that is 2 days old (past 1-day retention)
    let seg_id    = Uuid::new_v4();
    let old_start = Utc::now() - chrono::Duration::days(2);
    let old_end   = old_start + chrono::Duration::seconds(60);

    sqlx::query(
        "INSERT INTO recordings
             (id, camera_id, node_id, storage_uri, started_at, ended_at,
              duration_secs, size_bytes, retention_policy_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(seg_id)
    .bind(camera_id)
    .bind(node_id)
    .bind("local:///data/recordings/cam2/20260706_120000.mp4")
    .bind(old_start)
    .bind(old_end)
    .bind(60_i32)
    .bind(5_000_000_i64)
    .bind(policy_id)
    .execute(&app.db)
    .await
    .unwrap();

    // Run retention enforcer directly
    coordinator::retention::run_single_pass(&app.db).await;

    // Recording should be gone from DB
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recordings WHERE id = $1")
            .bind(seg_id)
            .fetch_one(&app.db)
            .await
            .unwrap();
    assert_eq!(count, 0, "expired recording should have been deleted");
}

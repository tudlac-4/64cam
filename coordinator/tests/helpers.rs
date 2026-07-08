use common::auth::hash_password;
use coordinator::{routes::create_router, state::AppState};
use reqwest::Client;
use sqlx::PgPool;
use std::net::SocketAddr;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

pub struct TestApp {
    pub db: PgPool,
    pub client: Client,
    pub addr: SocketAddr,
    // Keep container alive for the duration of the test
    _container: testcontainers::ContainerAsync<Postgres>,
}

impl TestApp {
    pub async fn spawn() -> Self {
        let container = Postgres::default().start().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let db_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

        let pool = PgPool::connect(&db_url).await.unwrap();
        sqlx::migrate!("../migrations").run(&pool).await.unwrap();

        let jwt_secret = b"test-secret-that-is-at-least-32-bytes!";
        let state = AppState::new(pool.clone(), jwt_secret);
        let app = create_router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        TestApp {
            db: pool,
            client: Client::new(),
            addr,
            _container: container,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    /// Seed an admin user and return (email, password, role_id).
    pub async fn seed_admin(&self) -> (String, String, Uuid) {
        let email = format!("admin-{}@test.local", Uuid::new_v4());
        let password = "Test1234!";
        let hash = hash_password(password).unwrap();
        let role_id: Uuid = sqlx::query_scalar!("SELECT id FROM roles WHERE name = 'admin'")
            .fetch_one(&self.db)
            .await
            .unwrap();
        sqlx::query!(
            "INSERT INTO users (email, password_hash, role_id) VALUES ($1, $2, $3)",
            email,
            hash,
            role_id
        )
        .execute(&self.db)
        .await
        .unwrap();
        (email, password.into(), role_id)
    }

    /// Login and return the access token.
    pub async fn login(&self, email: &str, password: &str) -> String {
        let resp = self
            .client
            .post(self.url("/api/v1/auth/login"))
            .json(&serde_json::json!({"email": email, "password": password}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "login failed");
        resp.json::<serde_json::Value>()
            .await
            .unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    pub fn bearer(&self, token: &str) -> reqwest::RequestBuilder {
        self.client
            .get(self.url("/"))  // placeholder; callers set the url
            .header("authorization", format!("Bearer {token}"))
    }
}

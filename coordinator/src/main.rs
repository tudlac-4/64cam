use std::net::SocketAddr;

use coordinator::{openapi::ApiDoc, retention, routes::create_router, state::AppState};
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url     = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let bind       = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/dist".into());

    let pool = sqlx::PgPool::connect(&db_url).await?;
    sqlx::migrate!("../migrations").run(&pool).await?;

    let state = AppState::new(pool, jwt_secret.as_bytes());

    tokio::spawn(retention::run_enforcer(state.clone()));

    // Serve the built frontend; unknown paths fall back to index.html for SPA routing
    let serve_dir = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(format!("{frontend_dir}/index.html")));

    let app = create_router(state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .fallback_service(serve_dir)
        // ConnectInfo is required for the node WS handler to capture the peer IP
        // and write it to nodes.ip_addr for cross-node playback proxy routing.
        .into_make_service_with_connect_info::<SocketAddr>();

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("listening on {bind}  (frontend: {frontend_dir})");
    axum::serve(listener, app).await?;
    Ok(())
}

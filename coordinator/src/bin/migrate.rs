use common::auth::hash_password;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;
    println!("migrations applied");

    // Seed default admin on first run (idempotent — skipped if any user exists).
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;

    if count == 0 {
        let hash = hash_password("admin")?;
        sqlx::query(
            "INSERT INTO users (email, password_hash, role_id)
             SELECT 'admin', $1, id FROM roles WHERE name = 'admin'",
        )
        .bind(&hash)
        .execute(&pool)
        .await?;
        println!("seeded default admin — username: admin  password: admin");
        println!("IMPORTANT: change the password after first login.");
    }

    Ok(())
}

use sqlx::PgPool;

pub async fn connect(url: &str) -> anyhow::Result<PgPool> {
    Ok(PgPool::connect(url).await?)
}

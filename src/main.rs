use dotenvy::dotenv;
use uptime_monitor::database::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;
    let db = std::env::var("DATABASE_URL").expect("Failed to load database url");
    let db = AppState::new(db)?;

    sqlx::migrate!()
        .run(db.connection())
        .await
        .expect("Migrations failed");

    Ok(())
}

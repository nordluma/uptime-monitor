use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;

use uptime_monitor::{
    database::AppState,
    domain::{create_website, delete_website, get_website_alias, get_websites},
    pages::styles,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;
    let db = std::env::var("DATABASE_URL").expect("Failed to load database url");

    let db = AppState::new(db)?;
    sqlx::migrate!()
        .run(db.connection())
        .await
        .expect("Migrations failed");

    let app = Router::new()
        .route("/", get(get_websites))
        .route("/websites", post(create_website))
        .route(
            "/websites/:alias",
            get(get_website_alias).delete(delete_website),
        )
        .route("/styles.css", get(styles))
        .with_state(db);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

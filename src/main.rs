use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
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

    let app = Router::new().route("/", get(hello_world)).with_state(db);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn hello_world() -> impl IntoResponse {
    Html("Hello world")
}

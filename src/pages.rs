use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub async fn styles() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../templates/styles.css").to_owned())
        .unwrap()
}

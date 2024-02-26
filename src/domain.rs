use std::time::Duration;

use anyhow::Context;
use askama_axum::IntoResponse;
use axum::{extract::State, http::StatusCode, response::Redirect, Form};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use sqlx::{prelude::FromRow, PgPool};
use tokio::time;
use validator::Validate;

use crate::{database::AppState, error::ApiError};

#[derive(Debug, Deserialize, FromRow, Validate)]
pub struct Website {
    #[validate(url)]
    url: String,
    alias: String,
}

pub async fn check_websites(db: PgPool) -> anyhow::Result<()> {
    let mut interval = time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        let ctx = Client::new();

        let mut res = sqlx::query_as!(Website, r#"SELECT url, alias FROM websites"#).fetch(&db);
        while let Some(website) = res.next().await {
            let website = website.with_context(|| "query for website failed")?;
            let response = ctx
                .get(&website.url)
                .send()
                .await
                .with_context(|| format!("failed to send request to: {}", website.url))?;

            sqlx::query!(
                r#"INSERT INTO logs (website_id, status)
            VALUES ((SELECT id FROM websites WHERE alias = $1), $2)
            "#,
                website.alias,
                response.status().as_u16() as i16
            )
            .execute(&db)
            .await
            .with_context(|| format!("Failed to insert log status for: {}", website.url))?;
        }
    }
}

pub async fn create_website(
    State(db): State<AppState>,
    Form(new_website): Form<Website>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if new_website.validate().is_err() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Validation Error: please check the URL",
        ));
    }

    sqlx::query!(
        r#"INSERT INTO websites (url, alias) VALUES ($1, $2)"#,
        new_website.url,
        new_website.alias
    )
    .execute(db.connection())
    .await
    .map_err(ApiError::SqlError)
    .unwrap();

    Ok(Redirect::to("/"))
}

use anyhow::Context;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    Form,
};
use chrono::{DateTime, Timelike, Utc};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
    let mut interval = time::interval(tokio::time::Duration::from_secs(60));

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

#[derive(Debug, Serialize, FromRow, Template)]
#[template(path = "index.html")]
struct WebsiteLogs {
    logs: Vec<WebsiteInfo>,
}

#[derive(Debug, Serialize, Validate)]
struct WebsiteInfo {
    #[validate(url)]
    url: String,
    alias: String,
    data: Vec<WebsiteStats>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WebsiteStats {
    time: DateTime<Utc>,
    uptime_pct: Option<i16>,
}

async fn get_websites(State(db): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let websites = sqlx::query_as!(Website, r#"SELECT url, alias FROM websites"#)
        .fetch_all(db.connection())
        .await?;

    let mut logs = Vec::with_capacity(websites.len());
    for website in websites {
        let data = get_daily_stats(&website.alias, db.connection()).await?;

        logs.push(WebsiteInfo {
            url: website.url,
            alias: website.alias,
            data,
        });
    }

    Ok(WebsiteLogs { logs })
}

enum SplitBy {
    Hour,
    Day,
}

async fn get_daily_stats(alias: &str, db: &PgPool) -> Result<Vec<WebsiteStats>, ApiError> {
    let data = sqlx::query_as::<_, WebsiteStats>(
        r#"SELECT date_trunc('hour', created_at) as time,
        CAST(COUNT(case when status = 200 then 1 end) * 100 / COUNT(*) as int2) as uptime_pct
        FROM logs
        LEFT JOIN websites on websites.id = logs.website_id
        WHERE websites.alias = $1
        GROUP BY time
        ORDER BY time ASC
        LIMIT 24
        "#,
    )
    .bind(alias)
    .fetch_all(db)
    .await?;

    let number_of_splits = 24;
    let number_of_seconds = 3600;

    let data = fill_data_gaps(data, number_of_splits, SplitBy::Hour, number_of_seconds);

    Ok(data)
}

fn fill_data_gaps(
    mut data: Vec<WebsiteStats>,
    splits: i32,
    format: SplitBy,
    number_of_seconds: i32,
) -> Vec<WebsiteStats> {
    if (data.len() as i32) < splits {
        for i in 1..24 {
            let time = Utc::now() - chrono::Duration::seconds((number_of_seconds * i).into());
            let time = time
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap();

            let time = if matches!(format, SplitBy::Day) {
                time.with_hour(0).unwrap()
            } else {
                time
            };

            if !data.iter().any(|x| x.time == time) {
                data.push(WebsiteStats {
                    time,
                    uptime_pct: None,
                });
            }
        }

        data.sort_by(|a, b| b.time.cmp(&a.time));
    }

    data
}

async fn get_monthly_stats(_alias: &str, _db: &PgPool) -> Result<Vec<WebsiteStats>, ApiError> {
    todo!()
}

#[derive(Debug, Serialize, FromRow, Template)]
#[template(path = "single_website.html")]
struct SingleWebsiteLogs {
    log: WebsiteInfo,
    incidents: Vec<Incident>,
    monthly_data: Vec<WebsiteStats>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Incident {
    time: DateTime<Utc>,
    status: i16,
}

async fn get_website_alias(
    State(db): State<AppState>,
    Path(alias): Path<String>,
) -> Result<impl askama_axum::IntoResponse, ApiError> {
    let website = sqlx::query_as!(
        Website,
        r#"SELECT url, alias FROM websites WHERE alias = $1"#,
        alias
    )
    .fetch_one(db.connection())
    .await?;

    let last_24_hours_data = get_daily_stats(&website.alias, db.connection()).await?;
    let monthly_data = get_monthly_stats(&website.alias, db.connection()).await?;

    let incidents = sqlx::query_as::<_, Incident>(
        r#"SELECT logs.created_at as time,
        logs.status FROM logs
        LEFT JOIN websites ON websites.id = logs.website_id
        WHERE website.alias = $1 and logs.status != 200"#,
    )
    .bind(&alias)
    .fetch_all(db.connection())
    .await?;

    let log = WebsiteInfo {
        url: website.url,
        alias,
        data: last_24_hours_data,
    };

    Ok(SingleWebsiteLogs {
        log,
        incidents,
        monthly_data,
    })
}

pub async fn delete_website(
    State(db): State<AppState>,
    Path(alias): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut tx = db.connection().begin().await?;
    if let Err(e) = sqlx::query!("DELETE FROM logs WHERE website_alias = $1", &alias)
        .execute(&mut *tx)
        .await
    {
        tx.rollback().await?;
        return Err(ApiError::SqlError(e));
    };

    if let Err(e) = sqlx::query!(r#"DELETE FROM websites WHERE alias = $1"#, alias)
        .execute(&mut *tx)
        .await
    {
        tx.rollback().await?;
        return Err(ApiError::SqlError(e));
    }

    tx.commit().await?;

    Ok(StatusCode::OK)
}

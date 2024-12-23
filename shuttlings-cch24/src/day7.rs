use axum::{
    body::Bytes,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use chrono::{offset::Utc, DateTime};
use rand::{distributions::DistString, rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;
use uuid::Uuid;

const PAGE_SIZE: i32 = 3;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DraftParams {
    author: String,
    quote: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListParams {
    token: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
struct Pagination {
    quotes: Vec<Quote>,
    page: i32,
    next_token: Option<String>,
}

#[derive(Serialize, Debug, Clone, FromRow)]
struct TokenRow {
    id: String,
    page: i32,
    prev_id: String,
}

#[derive(Serialize, Debug, Clone, FromRow)]
#[allow(clippy::struct_field_names)]
struct Quote {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<Utc>,
    version: i32,
}

impl From<DraftParams> for Quote {
    fn from(val: DraftParams) -> Self {
        Self {
            author: val.author,
            quote: val.quote,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            version: 1,
        }
    }
}

pub async fn reset(Extension(pool): Extension<Arc<PgPool>>) -> Result<(), (StatusCode, String)> {
    sqlx::query("DELETE FROM quotes")
        .execute(pool.as_ref())
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to execute query: {e:?}"),
            )
        })?;

    Ok(())
}

pub async fn draft(
    Extension(pool): Extension<Arc<PgPool>>,
    body: Bytes,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let params: DraftParams = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to deserialize payload: {e:?}"),
        )
    })?;

    let DraftParams { author, quote } = params;
    let id = Uuid::new_v4();
    let version = 1;

    let query = "
        INSERT INTO quotes (id, author, quote, version)
        VALUES ($1, $2, $3, $4)
        RETURNING id, author, quote, created_at, version
    ";

    // Insert the new row
    let quote = sqlx::query_as::<_, Quote>(query)
        .bind(id)
        .bind(author)
        .bind(quote)
        .bind(version)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to insert quote: {e:?}"),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        serde_json::to_string_pretty(&quote).unwrap(),
    ))
}

pub async fn cite(
    Extension(pool): Extension<Arc<PgPool>>,
    Path(id): Path<Uuid>,
) -> Result<String, (StatusCode, String)> {
    let query = "
        SELECT 
            * 
        FROM 
            quotes 
        WHERE
            id = $1
        LIMIT
            1
        ";

    // Insert the new row
    let quote = sqlx::query_as::<_, Quote>(query)
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("ID not found {id:?}: {e:?}")))?;

    Ok(serde_json::to_string_pretty(&quote).unwrap())
}

pub async fn remove(
    Extension(pool): Extension<Arc<PgPool>>,
    Path(id): Path<Uuid>,
) -> Result<String, (StatusCode, String)> {
    let query = "
        DELETE FROM 
            quotes 
        WHERE
            id = $1
        RETURNING 
            id, author, quote, created_at, version
        ";

    // Insert the new row
    let quote = sqlx::query_as::<_, Quote>(query)
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to delete {id:?}: {e:?}"),
            )
        })?;

    Ok(serde_json::to_string_pretty(&quote).unwrap())
}

async fn get_num_quotes(pool: &PgPool) -> Result<i32, (StatusCode, String)> {
    let query = "
        SELECT 
            COUNT(*)
        FROM
            quotes 
        ";

    let rows: (i64,) = sqlx::query_as(query)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Failed to list: {e:?}")))?;

    i32::try_from(rows.0).map_err(|e| (StatusCode::NOT_FOUND, format!("Too many rows: {e:?}")))
}

/// Update the next page for the given token
async fn update_token_page(
    pool: &PgPool,
    token: Option<String>,
) -> Result<String, (StatusCode, String)> {
    let mut rng = SmallRng::from_entropy();
    let token = token.unwrap_or(rand::distributions::Alphanumeric.sample_string(&mut rng, 16));

    let query = "
        INSERT INTO 
            pages (id, page)
        VALUES 
            ($1, 1)
        ON
            CONFLICT (id)
        DO UPDATE SET
            page = pages.page + 1
        ";

    sqlx::query(query)
        .bind(&token)
        .execute(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to create next token: {e:?}"),
            )
        })?;

    Ok(token)
}

/// Get the next page of quotes for the given token
async fn get_page_from_token(pool: &PgPool, token: &str) -> Result<i32, (StatusCode, String)> {
    let query = "
        SELECT
            page
        FROM
            pages
        where
            id = $1
        ";

    let page: (i32,) = sqlx::query_as(query)
        .bind(token)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Token not found: {e:?}")))?;

    Ok(page.0)
}

/// Get the next page of quotes for the given offset
async fn get_quotes_by_offset(
    pool: &PgPool,
    offset: i32,
) -> Result<Vec<Quote>, (StatusCode, String)> {
    let query = format!(
        "
        SELECT 
            *
        FROM
            quotes 
        ORDER BY 
            created_at ASC
        LIMIT
            {PAGE_SIZE}
        OFFSET
            $1
        "
    );

    let quotes = sqlx::query_as(&query)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Failed to list: {e:?}")))?;

    Ok(quotes)
}

pub async fn list(
    Extension(pool): Extension<Arc<PgPool>>,
    Query(ListParams { token }): Query<ListParams>,
) -> Result<String, (StatusCode, String)> {
    // Get the current page for the the given token
    let (page, token) = if let Some(token) = token {
        (
            get_page_from_token(pool.as_ref(), &token).await?,
            Some(token),
        )
    } else {
        (0, None)
    };

    let offset = page * PAGE_SIZE;

    let rows = get_num_quotes(pool.as_ref()).await?;

    let next_page = page + 1;
    let next_token = if rows > next_page * PAGE_SIZE {
        Some(update_token_page(&pool, token).await?)
    } else {
        None
    };

    let resp = Pagination {
        quotes: get_quotes_by_offset(&pool, offset).await?,
        page: next_page,
        next_token,
    };

    Ok(serde_json::to_string_pretty(&resp).unwrap())
}

pub async fn undo(
    Extension(pool): Extension<Arc<PgPool>>,
    Path(id): Path<Uuid>,
    body: Bytes,
) -> Result<String, (StatusCode, String)> {
    let params = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to deserialize payload: {e:?}"),
        )
    })?;

    let query = "
        UPDATE
            quotes
        SET
            author = $1, quote = $2, version = version + 1
        WHERE
            id = $3
        RETURNING 
            id, author, quote, created_at, version
        ";

    let DraftParams { author, quote } = params;

    // Insert the new row
    let quote = sqlx::query_as::<_, Quote>(query)
        .bind(author)
        .bind(quote)
        .bind(id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to delete {id:?}: {e:?}"),
            )
        })?;

    Ok(serde_json::to_string_pretty(&quote).unwrap())
}

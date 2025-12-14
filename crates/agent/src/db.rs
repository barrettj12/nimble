use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result};
use sqlx::{
    ConnectOptions,
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
};
use uuid::Uuid;

use crate::workers::build::BuildStatus;

/// Initialize the SQLite database connection pool.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    // Configure connection options
    let mut options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30));

    // Disable logging for connection events (optional, can be enabled for debugging)
    options = options.disable_statement_logging();

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(30))
        .connect_with(options)
        .await
        .context("Failed to create database connection pool")?;

    // Run migrations
    migrate(&pool).await?;

    Ok(pool)
}

/// Run database migrations to create necessary tables.
async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS builds (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create builds table")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_builds_status ON builds(status)
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create builds status index")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_builds_created_at ON builds(created_at)
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create builds created_at index")?;

    Ok(())
}

/// Insert a new build record into the database.
pub async fn create_build(pool: &SqlitePool, build_id: Uuid, status: BuildStatus) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO builds (id, status)
        VALUES (?1, ?2)
        "#,
    )
    .bind(build_id.to_string())
    .bind(status.as_str())
    .execute(pool)
    .await
    .context("Failed to insert build record")?;

    Ok(())
}

/// Update a build's status.
pub async fn update_build_status(
    pool: &SqlitePool,
    build_id: Uuid,
    status: BuildStatus,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE builds
        SET status = ?1, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?2
        "#,
    )
    .bind(status.as_str())
    .bind(build_id.to_string())
    .execute(pool)
    .await
    .context("Failed to update build status")?;

    Ok(())
}

/// Get a build by ID.
#[derive(Debug)]
pub struct BuildRecord {
    pub id: Uuid,
    pub status: BuildStatus,
    pub created_at: String,
    pub updated_at: String,
}

// Internal struct for SQLite row deserialization
#[derive(Debug, sqlx::FromRow)]
struct BuildRecordRow {
    id: String,
    status: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<BuildRecordRow> for BuildRecord {
    type Error = anyhow::Error;

    fn try_from(row: BuildRecordRow) -> Result<Self> {
        Ok(BuildRecord {
            id: Uuid::parse_str(&row.id).context("Failed to parse build ID as UUID")?,
            status: BuildStatus::from_str(&row.status)
                .map_err(|e| anyhow::anyhow!("Failed to parse build status: {e}"))?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn get_build(pool: &SqlitePool, build_id: Uuid) -> Result<Option<BuildRecord>> {
    let build = sqlx::query_as::<_, BuildRecordRow>(
        r#"
        SELECT id, status, created_at, updated_at
        FROM builds
        WHERE id = ?1
        "#,
    )
    .bind(build_id.to_string())
    .fetch_optional(pool)
    .await
    .context("Failed to fetch build record")?;

    build.map(BuildRecord::try_from).transpose()
}

/// List all builds, optionally filtered by status.
pub async fn list_builds(
    pool: &SqlitePool,
    limit: Option<i64>,
    status: Option<BuildStatus>,
) -> Result<Vec<BuildRecord>> {
    let mut query = String::from(
        r#"
        SELECT id, status, created_at, updated_at
        FROM builds
        "#,
    );

    if status.is_some() {
        query.push_str(" WHERE status = ?1");
    }

    query.push_str(" ORDER BY created_at DESC");

    if let Some(limit) = limit {
        query.push_str(&format!(" LIMIT {limit}"));
    }

    let builds = if let Some(status) = status {
        sqlx::query_as::<_, BuildRecordRow>(&query)
            .bind(status.as_str())
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, BuildRecordRow>(&query)
            .fetch_all(pool)
            .await
    }
    .context("Failed to fetch build records")?;

    builds
        .into_iter()
        .map(BuildRecord::try_from)
        .collect::<Result<Vec<_>>>()
}

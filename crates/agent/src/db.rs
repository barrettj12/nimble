use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result};
use sqlx::{
    ConnectOptions, Row,
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
};
use uuid::Uuid;

use crate::workers::{build::BuildStatus, deploy::DeployStatus};

/// Lightweight wrapper around the SQLx pool to encapsulate DB access.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Initialize the SQLite database connection pool and run migrations.
    pub async fn connect(database_url: &str) -> Result<Self> {
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

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    /// Insert a new build record into the database.
    pub async fn create_build(&self, build_id: Uuid, status: BuildStatus) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO builds (id, status)
            VALUES (?1, ?2)
            "#,
        )
        .bind(build_id.to_string())
        .bind(status.as_str())
        .execute(&self.pool)
        .await
        .context("Failed to insert build record")?;

        Ok(())
    }

    /// Update a build's status.
    pub async fn update_build_status(&self, build_id: Uuid, status: BuildStatus) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE builds
            SET status = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            "#,
        )
        .bind(status.as_str())
        .bind(build_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update build status")?;

        Ok(())
    }

    /// Insert a new deployment record.
    pub async fn create_deployment(
        &self,
        deploy_id: Uuid,
        build_id: Uuid,
        app: &str,
        image_reference: &str,
        status: DeployStatus,
        address: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO deployments (id, build_id, app, image, status, address)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(deploy_id.to_string())
        .bind(build_id.to_string())
        .bind(app)
        .bind(image_reference)
        .bind(status.as_str())
        .bind(address)
        .execute(&self.pool)
        .await
        .context("Failed to insert deployment record")?;

        Ok(())
    }

    /// Ensure an app record exists.
    pub async fn upsert_app(&self, app: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO apps (name)
            VALUES (?1)
            ON CONFLICT(name) DO NOTHING
            "#,
        )
        .bind(app)
        .execute(&self.pool)
        .await
        .context("Failed to upsert app")?;

        Ok(())
    }

    /// Update the active deployment for an app.
    pub async fn set_active_deployment(&self, app: &str, deploy_id: Option<Uuid>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE apps
            SET active_deployment_id = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE name = ?2
            "#,
        )
        .bind(deploy_id.map(|id| id.to_string()))
        .bind(app)
        .execute(&self.pool)
        .await
        .context("Failed to update app active deployment")?;

        Ok(())
    }

    /// Get app details by name.
    pub async fn get_app(&self, app: &str) -> Result<Option<AppRecord>> {
        let record = sqlx::query_as::<_, AppRecordRow>(
            r#"
            SELECT name, active_deployment_id, created_at, updated_at
            FROM apps
            WHERE name = ?1
            "#,
        )
        .bind(app)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch app record")?;

        record.map(AppRecord::try_from).transpose()
    }

    /// Fetch the active deployment ID for an app if present.
    pub async fn get_active_deployment_id(&self, app: &str) -> Result<Option<Uuid>> {
        Ok(self
            .get_app(app)
            .await?
            .and_then(|app| app.active_deployment_id))
    }

    /// Update deployment status.
    pub async fn update_deployment_status(
        &self,
        deploy_id: Uuid,
        status: DeployStatus,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE deployments
            SET status = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            "#,
        )
        .bind(status.as_str())
        .bind(deploy_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update deployment status")?;

        Ok(())
    }

    /// Store container details for a deployment.
    pub async fn set_deployment_container(
        &self,
        deploy_id: Uuid,
        container_id: &str,
        container_name: &str,
        address: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE deployments
            SET container_id = ?1, container_name = ?2, address = ?3, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?4
            "#,
        )
        .bind(container_id)
        .bind(container_name)
        .bind(address)
        .bind(deploy_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to record deployment container details")?;

        Ok(())
    }

    pub async fn get_build(&self, build_id: Uuid) -> Result<Option<BuildRecord>> {
        let build = sqlx::query_as::<_, BuildRecordRow>(
            r#"
            SELECT id, status, created_at, updated_at
            FROM builds
            WHERE id = ?1
            "#,
        )
        .bind(build_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch build record")?;

        build.map(BuildRecord::try_from).transpose()
    }

    /// List all builds, optionally filtered by status.
    pub async fn list_builds(
        &self,
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
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, BuildRecordRow>(&query)
                .fetch_all(&self.pool)
                .await
        }
        .context("Failed to fetch build records")?;

        builds
            .into_iter()
            .map(BuildRecord::try_from)
            .collect::<Result<Vec<_>>>()
    }

    /// Run database migrations to create necessary tables.
    async fn migrate(&self) -> Result<()> {
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
        .execute(&self.pool)
        .await
        .context("Failed to create builds table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_builds_status ON builds(status)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create builds status index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_builds_created_at ON builds(created_at)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create builds created_at index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deployments (
                id TEXT PRIMARY KEY,
                build_id TEXT NOT NULL,
                app TEXT NOT NULL,
                image TEXT NOT NULL,
                status TEXT NOT NULL,
                container_id TEXT,
                container_name TEXT,
                address TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create deployments table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS apps (
                name TEXT PRIMARY KEY,
                active_deployment_id TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create apps table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_apps_active_deployment ON apps(active_deployment_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create apps active_deployment index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create deployments status index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_deployments_created_at ON deployments(created_at)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create deployments created_at index")?;

        // Add address column if missing (SQLite lacks IF NOT EXISTS)
        if !self
            .deployment_column_exists("address")
            .await
            .context("Failed to inspect deployments table for address column")?
        {
            let _ = sqlx::query(
                r#"
                ALTER TABLE deployments ADD COLUMN address TEXT
                "#,
            )
            .execute(&self.pool)
            .await;
        }

        // Add app column if missing (SQLite lacks IF NOT EXISTS)
        if !self
            .deployment_column_exists("app")
            .await
            .context("Failed to inspect deployments table for app column")?
        {
            let _ = sqlx::query(
                r#"
                ALTER TABLE deployments ADD COLUMN app TEXT
                "#,
            )
            .execute(&self.pool)
            .await;

            let _ = sqlx::query(
                r#"
                UPDATE deployments SET app = 'unknown' WHERE app IS NULL
                "#,
            )
            .execute(&self.pool)
            .await;
        }

        Ok(())
    }
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

/// Application record.
#[derive(Debug)]
#[allow(dead_code)]
pub struct AppRecord {
    pub name: String,
    pub active_deployment_id: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

// Internal struct for SQLite row deserialization
#[derive(Debug, sqlx::FromRow)]
struct AppRecordRow {
    name: String,
    active_deployment_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<AppRecordRow> for AppRecord {
    type Error = anyhow::Error;

    fn try_from(row: AppRecordRow) -> Result<Self> {
        Ok(AppRecord {
            name: row.name,
            active_deployment_id: row
                .active_deployment_id
                .map(|id| Uuid::parse_str(&id))
                .transpose()
                .context("Failed to parse app active deployment as UUID")?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Deployment record.
#[derive(Debug)]
pub struct DeploymentRecord {
    pub id: Uuid,
    pub build_id: Uuid,
    pub app: String,
    pub image: String,
    pub status: DeployStatus,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// Internal struct for SQLite row deserialization
#[derive(Debug, sqlx::FromRow)]
struct DeploymentRecordRow {
    id: String,
    build_id: String,
    app: String,
    image: String,
    status: String,
    container_id: Option<String>,
    container_name: Option<String>,
    address: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<DeploymentRecordRow> for DeploymentRecord {
    type Error = anyhow::Error;

    fn try_from(row: DeploymentRecordRow) -> Result<Self> {
        Ok(DeploymentRecord {
            id: Uuid::parse_str(&row.id).context("Failed to parse deployment ID as UUID")?,
            build_id: Uuid::parse_str(&row.build_id)
                .context("Failed to parse deployment build ID as UUID")?,
            app: row.app,
            image: row.image,
            status: DeployStatus::from_str(&row.status)
                .map_err(|e| anyhow::anyhow!("Failed to parse deployment status: {e}"))?,
            container_id: row.container_id,
            container_name: row.container_name,
            address: row.address,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl Database {
    /// Fetch a deployment by ID.
    pub async fn get_deployment(&self, deploy_id: Uuid) -> Result<Option<DeploymentRecord>> {
        let deployment = sqlx::query_as::<_, DeploymentRecordRow>(
            r#"
            SELECT id, build_id, app, image, status, container_id, container_name, address, created_at, updated_at
            FROM deployments
            WHERE id = ?1
            "#,
        )
        .bind(deploy_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch deployment record")?;

        deployment.map(DeploymentRecord::try_from).transpose()
    }

    /// List deployments, optionally filtered by build_id.
    pub async fn list_deployments(&self, build_id: Option<Uuid>) -> Result<Vec<DeploymentRecord>> {
        let mut query = String::from(
            r#"
            SELECT id, build_id, app, image, status, container_id, container_name, address, created_at, updated_at
            FROM deployments
            "#,
        );

        if build_id.is_some() {
            query.push_str(" WHERE build_id = ?1");
        }

        query.push_str(" ORDER BY created_at DESC");

        let deployments = if let Some(build_id) = build_id {
            sqlx::query_as::<_, DeploymentRecordRow>(&query)
                .bind(build_id.to_string())
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, DeploymentRecordRow>(&query)
                .fetch_all(&self.pool)
                .await
        }
        .context("Failed to fetch deployment records")?;

        deployments
            .into_iter()
            .map(DeploymentRecord::try_from)
            .collect::<Result<Vec<_>>>()
    }

    async fn deployment_column_exists(&self, column_name: &str) -> Result<bool> {
        let rows = sqlx::query(
            r#"
            PRAGMA table_info('deployments')
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to inspect deployments table")?;

        for row in rows {
            let name: String = row.try_get("name")?;
            if name == column_name {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

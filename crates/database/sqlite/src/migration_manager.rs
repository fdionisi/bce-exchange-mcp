use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub struct MigrationManager;

impl MigrationManager {
    pub async fn ensure_current_schema(pool: &SqlitePool) -> Result<()> {
        Self::create_version_table(pool).await?;

        let current_version = Self::get_current_version(pool).await?;
        let target_version = Self::get_target_version();

        if current_version < target_version {
            Self::apply_migrations(pool, current_version, target_version).await?;
        }

        Ok(())
    }

    async fn create_version_table(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER DEFAULT (strftime('%s', 'now'))
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create version table: {}", e))?;

        Ok(())
    }

    async fn get_current_version(pool: &SqlitePool) -> Result<i32> {
        let version: Option<i32> = sqlx::query_scalar("SELECT MAX(version) FROM schema_version")
            .fetch_optional(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database operation failed: {}", e))?
            .flatten();

        Ok(version.unwrap_or(0))
    }

    fn get_target_version() -> i32 {
        MIGRATIONS.len() as i32
    }

    async fn apply_migrations(pool: &SqlitePool, from: i32, to: i32) -> Result<()> {
        for version in (from + 1)..=to {
            let migration_index = (version - 1) as usize;
            if let Some(migration) = MIGRATIONS.get(migration_index) {
                Self::apply_migration(pool, version, migration).await?;
            }
        }
        Ok(())
    }

    async fn apply_migration(pool: &SqlitePool, version: i32, migration: &Migration) -> Result<()> {
        let mut tx = pool.begin().await.map_err(anyhow::Error::from)?;

        sqlx::query(migration.sql)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Migration {} failed: {}", version, e))?;

        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(version)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to record migration version {}: {}", version, e)
            })?;

        tx.commit()
            .await
            .map_err(|e| anyhow::anyhow!("Operation failed: {}", e))?;

        Ok(())
    }
}

struct Migration {
    #[allow(unused)]
    description: &'static str,
    sql: &'static str,
}

static MIGRATIONS: &[Migration] = &[Migration {
    description: "Initial schema - exchange rates table",
    sql: include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/20250810162329_initial_schema.sql"
    )),
}];

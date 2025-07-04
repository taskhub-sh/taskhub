use dirs;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tokio::fs;

pub mod models;
pub mod operations;

pub async fn init_db(db_path: Option<PathBuf>) -> Result<SqlitePool, sqlx::Error> {
    let db_url = if let Some(path) = db_path {
        if path.to_str() == Some(":memory:") {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}", path.to_str().unwrap())
        }
    } else {
        let path = get_default_db_path().expect("Could not determine default database path");
        format!("sqlite://{}", path.to_str().unwrap())
    };

    if !db_url.contains(":memory:") {
        if let Some(parent) = PathBuf::from(db_url.strip_prefix("sqlite://").unwrap()).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await.map_err(sqlx::Error::Io)?;
            }
        }
    }

    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create the tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL,
            external_id TEXT,
            source TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            priority TEXT NOT NULL,
            assignee TEXT,
            labels TEXT,
            due_date TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            custom_fields TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn get_default_db_path() -> Option<PathBuf> {
    // Try data_dir first (preferred location)
    if let Some(mut path) = dirs::data_dir() {
        path.push("taskhub");
        path.push("taskhub.db");
        return Some(path);
    }

    // Fallback to home directory if data_dir is not available
    if let Some(mut path) = dirs::home_dir() {
        path.push(".local");
        path.push("share");
        path.push("taskhub");
        path.push("taskhub.db");
        return Some(path);
    }

    // Final fallback to current directory (for CI environments)
    let mut path = std::env::current_dir().ok()?;
    path.push("taskhub.db");
    Some(path)
}

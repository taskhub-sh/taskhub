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
    Ok(pool)
}

fn get_default_db_path() -> Option<PathBuf> {
    let mut path = dirs::data_dir()?;
    path.push("taskhub");
    path.push("taskhub.db");
    Some(path)
}

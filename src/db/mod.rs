use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use std::path::PathBuf;
use dirs;
use tokio::fs;

pub mod models;
pub mod operations;

pub async fn init_db(db_path: Option<PathBuf>) -> Result<SqlitePool, sqlx::Error> {
    let db_file_path = if let Some(path) = db_path {
        path
    } else {
        get_default_db_path().expect("Could not determine default database path")
    };

    let db_url = format!("sqlite://{}", db_file_path.to_str().unwrap());

    if let Some(parent) = db_file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await.map_err(|e| sqlx::Error::Io(e))?;
        }
    }

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url).await?;
    }
    let pool = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./src/db/migrations").run(&pool).await?;
    Ok(pool)
}

fn get_default_db_path() -> Option<PathBuf> {
    let mut path = dirs::data_dir()?;
    path.push("taskhub");
    path.push("taskhub.db");
    Some(path)
}

use sqlx::{Row, SqlitePool};

#[derive(Debug)]
pub struct HistoryManager {
    db_pool: SqlitePool,
    max_entries: usize,
}

impl HistoryManager {
    pub fn new(db_pool: SqlitePool, max_entries: Option<usize>) -> Self {
        Self {
            db_pool,
            max_entries: max_entries.unwrap_or(1000),
        }
    }

    pub async fn load_history(&self) -> Vec<String> {
        let query = r#"
            SELECT command
                FROM command_history
                ORDER BY created_at ASC
                LIMIT ?
        "#;

        match sqlx::query(query)
            .bind(self.max_entries as i64)
            .fetch_all(&self.db_pool)
            .await
        {
            Ok(rows) => {
                let mut commands = Vec::new();
                for row in rows {
                    commands.push(row.get("command"));
                }
                commands
            }
            Err(e) => {
                eprintln!("Warning: Failed to load command history: {e}");
                Vec::new()
            }
        }
    }

    pub async fn save_history(&self, history: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        // Clear existing history first
        sqlx::query("DELETE FROM command_history")
            .execute(&self.db_pool)
            .await?;

        // Insert entries (limit to max_entries)
        let entries_to_save = if history.len() > self.max_entries {
            &history[history.len() - self.max_entries..]
        } else {
            history
        };

        for command in entries_to_save {
            sqlx::query(
                r#"
                INSERT INTO command_history (command)
                VALUES (?)
            "#,
            )
            .bind(command)
            .execute(&self.db_pool)
            .await?;
        }

        Ok(())
    }

    pub async fn append_command(&self, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Insert the new command
        sqlx::query(
            r#"
            INSERT INTO command_history (command)
            VALUES (?)
        "#,
        )
        .bind(command)
        .execute(&self.db_pool)
        .await?;

        // Clean up old entries if we exceed max_entries
        // First, count current entries
        let count_result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM command_history")
            .fetch_one(&self.db_pool)
            .await?;

        if count_result > self.max_entries as i64 {
            // Delete oldest entries to keep only max_entries
            let to_delete = count_result - self.max_entries as i64;
            sqlx::query(
                r#"
                DELETE FROM command_history
                WHERE id IN (
                    SELECT id FROM command_history
                    ORDER BY id ASC
                    LIMIT ?
                )
            "#,
            )
            .bind(to_delete)
            .execute(&self.db_pool)
            .await?;
        }

        Ok(())
    }

    pub async fn clear_history(&self) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM command_history")
            .execute(&self.db_pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create the command_history table
        sqlx::query(
            r#"
            CREATE TABLE command_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
        "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_history_manager_new() {
        let pool = setup_test_db().await;
        let manager = HistoryManager::new(pool, Some(500));

        assert_eq!(manager.max_entries, 500);
    }

    #[tokio::test]
    async fn test_save_and_load_history() {
        let pool = setup_test_db().await;
        let manager = HistoryManager::new(pool, Some(100));

        let commands = vec!["ls".to_string(), "pwd".to_string()];

        manager.save_history(&commands).await.unwrap();
        let loaded = manager.load_history().await;

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0], "ls");
        assert_eq!(loaded[1], "pwd");
    }

    #[tokio::test]
    async fn test_append_command() {
        let pool = setup_test_db().await;
        let manager = HistoryManager::new(pool, Some(2));

        manager.append_command("echo hello").await.unwrap();
        let history1 = manager.load_history().await;
        assert_eq!(history1.len(), 1);

        manager.append_command("echo world").await.unwrap();
        let history2 = manager.load_history().await;
        assert_eq!(history2.len(), 2);

        manager.append_command("echo overflow").await.unwrap();
        let history3 = manager.load_history().await;
        assert_eq!(history3.len(), 2); // Should be limited to max_entries

        // After the third insert, we should have the two most recent entries
        assert_eq!(history3[0], "echo world");
        assert_eq!(history3[1], "echo overflow");
    }

    #[tokio::test]
    async fn test_clear_history() {
        let pool = setup_test_db().await;
        let manager = HistoryManager::new(pool, Some(100));

        manager.append_command("test").await.unwrap();
        assert_eq!(manager.load_history().await.len(), 1);

        manager.clear_history().await.unwrap();
        assert_eq!(manager.load_history().await.len(), 0);
    }
}

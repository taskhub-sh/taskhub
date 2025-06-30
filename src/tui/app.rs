use crate::db::models::Task;
use crate::db::operations;
use sqlx::SqlitePool;

pub struct App {
    pub should_quit: bool,
    pub db_pool: SqlitePool,
    pub tasks: Vec<Task>,
}

impl App {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self {
            should_quit: false,
            db_pool,
            tasks: Vec::new(),
        }
    }

    pub async fn load_tasks(&mut self) -> Result<(), sqlx::Error> {
        self.tasks = operations::list_tasks(&self.db_pool).await?;
        Ok(())
    }

    pub fn on_key(&mut self, key: char) {
        if key == 'q' {
            self.should_quit = true;
        }
    }
}

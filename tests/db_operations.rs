use std::collections::HashMap;
use std::fs;
use taskhub::db::init_db;
use taskhub::db::models::{Priority, Task, TaskSource, TaskStatus};
use taskhub::db::operations::{create_task, delete_task, get_task, list_tasks, update_task};
use uuid::Uuid;

#[tokio::test]
async fn test_db_operations() -> Result<(), Box<dyn std::error::Error>> {
    let pool = init_db(Some(":memory:".into())).await?;

    // Run migrations
    let migration_files = fs::read_dir("./src/db/migrations")?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    for file in migration_files {
        let sql = fs::read_to_string(file)?;
        sqlx::query(&sql).execute(&pool).await?;
    }

    // Create a task
    let new_task = Task {
        id: Uuid::new_v4(),
        external_id: None,
        source: TaskSource::Markdown,
        title: "Test Task".to_string(),
        description: Some("This is a test task.".to_string()),
        status: TaskStatus::Open,
        priority: Priority::Medium,
        assignee: None,
        labels: vec!["test".to_string()],
        due_date: None,
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
        custom_fields: HashMap::new(),
    };
    create_task(&pool, &new_task).await?;

    // Get the task
    let fetched_task = get_task(&pool, new_task.id).await?;
    assert_eq!(fetched_task.title, "Test Task");

    // Update the task
    let updated_task = Task {
        status: TaskStatus::Done,
        ..fetched_task
    };
    update_task(&pool, &updated_task).await?;
    let fetched_updated_task = get_task(&pool, new_task.id).await?;
    assert!(matches!(fetched_updated_task.status, TaskStatus::Done));

    // List tasks
    let tasks = list_tasks(&pool).await?;
    assert!(!tasks.is_empty());

    // Delete the task
    delete_task(&pool, new_task.id).await?;
    let result = get_task(&pool, new_task.id).await;
    assert!(result.is_err());

    Ok(())
}

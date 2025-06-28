use super::models::Task;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn create_task(pool: &SqlitePool, task: &Task) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks (id, external_id, source, title, description, status, priority, assignee, labels, due_date, created_at, updated_at, custom_fields) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task.id.to_string())
    .bind(&task.external_id)
    .bind(serde_json::to_string(&task.source).unwrap())
    .bind(&task.title)
    .bind(&task.description)
    .bind(serde_json::to_string(&task.status).unwrap())
    .bind(serde_json::to_string(&task.priority).unwrap())
    .bind(&task.assignee)
    .bind(serde_json::to_string(&task.labels).unwrap())
    .bind(&task.due_date)
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .bind(serde_json::to_string(&task.custom_fields).unwrap())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_task(pool: &SqlitePool, id: Uuid) -> Result<Task, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(pool)
        .await?;
    let task = Task {
        id: Uuid::parse_str(row.get("id")).unwrap(),
        external_id: row.get("external_id"),
        source: serde_json::from_str(row.get("source")).unwrap(),
        title: row.get("title"),
        description: row.get("description"),
        status: serde_json::from_str(row.get("status")).unwrap(),
        priority: serde_json::from_str(row.get("priority")).unwrap(),
        assignee: row.get("assignee"),
        labels: serde_json::from_str(row.get("labels")).unwrap(),
        due_date: row.get("due_date"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        custom_fields: serde_json::from_str(row.get("custom_fields")).unwrap(),
    };
    Ok(task)
}

pub async fn update_task(pool: &SqlitePool, task: &Task) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET external_id = ?, source = ?, title = ?, description = ?, status = ?, priority = ?, assignee = ?, labels = ?, due_date = ?, updated_at = ?, custom_fields = ? WHERE id = ?",
    )
    .bind(&task.external_id)
    .bind(serde_json::to_string(&task.source).unwrap())
    .bind(&task.title)
    .bind(&task.description)
    .bind(serde_json::to_string(&task.status).unwrap())
    .bind(serde_json::to_string(&task.priority).unwrap())
    .bind(&task.assignee)
    .bind(serde_json::to_string(&task.labels).unwrap())
    .bind(&task.due_date)
    .bind(&task.updated_at)
    .bind(serde_json::to_string(&task.custom_fields).unwrap())
    .bind(task.id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_task(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM tasks").fetch_all(pool).await?;

    let tasks: Vec<Task> = rows
        .into_iter()
        .map(|row| Task {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            external_id: row.get("external_id"),
            source: serde_json::from_str(row.get("source")).unwrap(),
            title: row.get("title"),
            description: row.get("description"),
            status: serde_json::from_str(row.get("status")).unwrap(),
            priority: serde_json::from_str(row.get("priority")).unwrap(),
            assignee: row.get("assignee"),
            labels: serde_json::from_str(row.get("labels")).unwrap(),
            due_date: row.get("due_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            custom_fields: serde_json::from_str(row.get("custom_fields")).unwrap(),
        })
        .collect();

    Ok(tasks)
}

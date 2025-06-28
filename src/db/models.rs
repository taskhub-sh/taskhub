use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskSource {
    Jira,
    GitHub,
    GitLab,
    Markdown,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub source: TaskSource,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub custom_fields: HashMap<String, String>,
}

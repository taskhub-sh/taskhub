use reqwest::{Client, header};
use serde::Deserialize;
use crate::db::models::{Task, TaskSource, TaskStatus, Priority};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct GitHubIssue {
    
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    assignee: Option<GitHubUser>,
    labels: Vec<GitHubLabel>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: String,
}

pub struct GitHubClient {
    client: Client,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("token {}", token)).unwrap(),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("TaskHub"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            base_url: "https://api.github.com".to_string(),
        }
    }

    pub async fn fetch_issues(&self, owner: &str, repo: &str) -> Result<Vec<Task>, reqwest::Error> {
        let url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);
        let issues: Vec<GitHubIssue> = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let tasks: Vec<Task> = issues.into_iter().map(|issue| {
            let status = match issue.state.as_str() {
                "open" => TaskStatus::Open,
                "closed" => TaskStatus::Done,
                _ => TaskStatus::Open, // Default to open
            };

            let assignee = issue.assignee.map(|u| u.login);
            let labels = issue.labels.into_iter().map(|l| l.name).collect();

            Task {
                id: Uuid::new_v4(), // Generate a new UUID for internal ID
                external_id: Some(issue.number.to_string()),
                source: TaskSource::GitHub,
                title: issue.title,
                description: issue.body,
                status,
                priority: Priority::Medium, // GitHub issues don't have direct priority, default to medium
                assignee,
                labels,
                due_date: None,
                created_at: issue.created_at,
                updated_at: issue.updated_at,
                custom_fields: HashMap::new(),
            }
        }).collect();

        Ok(tasks)
    }
}

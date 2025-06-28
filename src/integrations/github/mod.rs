use reqwest::{Client, header};

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

    pub async fn fetch_issues(&self) -> Result<(), reqwest::Error> {
        // Placeholder for fetching issues
        Ok(())
    }
}

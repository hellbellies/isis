//! Minimal Jira REST client: authentication, error mapping and paged JQL search.

use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::jira::{JiraIssue, JiraUser};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const PAGE_SIZE: u32 = 100;
/// Safety stop against a server that keeps returning page tokens.
const MAX_PAGES: usize = 500;

/// Everything needed to authenticate against a Jira site.
pub struct JiraCredentials {
    pub base_url: String,
    pub email: String,
    pub token: String,
}

pub struct JiraClient {
    http: reqwest::Client,
    credentials: JiraCredentials,
}

impl JiraClient {
    pub fn new(credentials: JiraCredentials) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|err| AppError::Jira(format!("Could not create HTTP client: {err}")))?;
        Ok(Self { http, credentials })
    }

    pub fn base_url(&self) -> &str {
        &self.credentials.base_url
    }

    pub fn email(&self) -> &str {
        &self.credentials.email
    }

    /// `GET /rest/api/2/myself` — the authenticated user. Available on Cloud and Data Center.
    pub async fn myself(&self) -> AppResult<JiraUser> {
        let response = self
            .send(self.http.get(self.url("/rest/api/2/myself")))
            .await?;
        self.parse(response).await
    }

    /// Runs a JQL search via `POST /rest/api/3/search/jql`, following all pages.
    /// Requests the `summary` and `updated` fields.
    pub async fn search(&self, jql: &str) -> AppResult<Vec<JiraIssue>> {
        let mut issues = Vec::new();
        let mut next_page_token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let body = SearchRequest {
                jql,
                fields: &["summary", "updated"],
                max_results: PAGE_SIZE,
                next_page_token: next_page_token.as_deref(),
            };
            let request = self
                .http
                .post(self.url("/rest/api/3/search/jql"))
                .json(&body);
            let page: SearchPage = self.parse(self.send(request).await?).await?;
            issues.extend(page.issues.into_iter().map(JiraIssue::from));
            match page.next_page_token {
                Some(token) if !page.is_last => next_page_token = Some(token),
                _ => return Ok(issues),
            }
        }
        Err(AppError::Jira(format!(
            "Search returned more than {MAX_PAGES} pages; aborting"
        )))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.credentials.base_url)
    }

    /// Sends an authenticated request and maps HTTP failures to user-facing messages.
    async fn send(&self, request: RequestBuilder) -> AppResult<Response> {
        let base_url = self.base_url();
        let response = request
            .basic_auth(&self.credentials.email, Some(&self.credentials.token))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|err| {
                AppError::Jira(format!("Could not reach {base_url}: {}", root_cause(&err)))
            })?;

        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let message = match status {
            StatusCode::UNAUTHORIZED => "Jira rejected the email or API token".to_owned(),
            StatusCode::FORBIDDEN => {
                "Access denied. Log in to Jira in the browser once (it may require a CAPTCHA), then retry".to_owned()
            }
            StatusCode::NOT_FOUND => format!("No Jira REST API found at {base_url}"),
            _ => match response.json::<ErrorBody>().await {
                Ok(body) if !body.error_messages.is_empty() => {
                    format!("Jira: {}", body.error_messages.join(" "))
                }
                _ => format!("Jira responded with {status}"),
            },
        };
        Err(AppError::Jira(message))
    }

    async fn parse<T: for<'de> Deserialize<'de>>(&self, response: Response) -> AppResult<T> {
        response.json::<T>().await.map_err(|_| {
            AppError::Jira(format!(
                "{} did not respond like a Jira site",
                self.base_url()
            ))
        })
    }
}

/// The innermost error is usually the useful one (DNS failure, refused connection, timeout…).
fn root_cause(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        return "the request timed out".into();
    }
    let mut cause: &dyn std::error::Error = err;
    while let Some(source) = cause.source() {
        cause = source;
    }
    cause.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest<'a> {
    jql: &'a str,
    fields: &'a [&'a str],
    max_results: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page_token: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPage {
    #[serde(default)]
    issues: Vec<SearchIssue>,
    next_page_token: Option<String>,
    #[serde(default)]
    is_last: bool,
}

#[derive(Deserialize)]
struct SearchIssue {
    id: String,
    key: String,
    #[serde(default)]
    fields: SearchFields,
}

#[derive(Deserialize, Default)]
struct SearchFields {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    updated: String,
}

impl From<SearchIssue> for JiraIssue {
    fn from(issue: SearchIssue) -> Self {
        Self {
            id: issue.id,
            key: issue.key,
            summary: issue.fields.summary,
            updated: issue.fields.updated,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    #[serde(default)]
    error_messages: Vec<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use mockito::Matcher;

    use super::*;

    pub const SEARCH: &str = "/rest/api/3/search/jql";
    const MYSELF: &str = "/rest/api/2/myself";
    const AUTH: &str = "Basic bWVAZXhhbXBsZS5jb206c2VjcmV0"; // me@example.com:secret

    pub fn client(base_url: &str) -> JiraClient {
        JiraClient::new(JiraCredentials {
            base_url: base_url.into(),
            email: "me@example.com".into(),
            token: "secret".into(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn myself_returns_user_on_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", MYSELF)
            .match_header("authorization", AUTH)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"displayName":"Ada Lovelace","emailAddress":"me@example.com","accountId":"1"}"#,
            )
            .create_async()
            .await;

        let user = client(&server.url()).myself().await.unwrap();
        assert_eq!(user.display_name, "Ada Lovelace");
        assert_eq!(user.email_address.as_deref(), Some("me@example.com"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn failures_map_to_messages() {
        let mut server = mockito::Server::new_async().await;
        for (status, body, expected) in [
            (401, "", "rejected the email or API token"),
            (404, "", "No Jira REST API"),
            (
                200,
                "<html>hello</html>",
                "did not respond like a Jira site",
            ),
            (500, "", "responded with 500"),
            (
                400,
                r#"{"errorMessages":["The value 'NOPE' does not exist for the field 'project'."]}"#,
                "Jira: The value 'NOPE' does not exist",
            ),
        ] {
            server.reset();
            server
                .mock("GET", MYSELF)
                .with_status(status)
                .with_body(body)
                .create_async()
                .await;
            match client(&server.url()).myself().await {
                Err(AppError::Jira(message)) => assert!(message.contains(expected), "{message}"),
                other => panic!("expected Jira error for {status}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn search_follows_page_tokens() {
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("POST", SEARCH)
            .match_header("authorization", AUTH)
            .match_body(Matcher::Regex(r#""maxResults":100\}$"#.into()))
            .with_body(
                r#"{"issues":[{"id":"1","key":"FOO-1","fields":{"summary":"One","updated":"2026-11-01T10:00:00.000+0000"}}],"nextPageToken":"p2","isLast":false}"#,
            )
            .create_async()
            .await;
        let second = server
            .mock("POST", SEARCH)
            .match_body(Matcher::PartialJsonString(r#"{"nextPageToken":"p2"}"#.into()))
            .with_body(
                r#"{"issues":[{"id":"2","key":"FOO-2","fields":{"summary":"Two","updated":"2026-11-02T10:00:00.000+0000"}}],"isLast":true}"#,
            )
            .create_async()
            .await;

        let issues = client(&server.url()).search("project = FOO").await.unwrap();
        let keys: Vec<_> = issues.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(keys, ["FOO-1", "FOO-2"]);
        assert_eq!(issues[1].summary, "Two");
        first.assert_async().await;
        second.assert_async().await;
    }
}

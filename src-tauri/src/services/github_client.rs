//! Minimal GitHub GraphQL client: authentication, error mapping, pull request
//! search and lookup by node id.

use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::{AppError, AppResult};
use crate::models::github::{GitHubUser, PullRequest, PullRequestState, ReviewDecision};

pub const GRAPHQL_URL: &str = "https://api.github.com/graphql";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// GitHub search returns at most 1000 results (10 pages of 100).
const MAX_SEARCH_PAGES: usize = 10;
/// Maximum ids per `nodes(ids:)` query.
const NODES_CHUNK: usize = 100;

const PR_FIELDS: &str = "id number title url state isDraft reviewDecision updatedAt \
                         repository { nameWithOwner } author { login }";

pub struct GitHubClient {
    http: reqwest::Client,
    endpoint: String,
    token: String,
}

impl GitHubClient {
    pub fn new(token: String) -> AppResult<Self> {
        Self::with_endpoint(GRAPHQL_URL.to_owned(), token)
    }

    /// Client for a different GraphQL endpoint (tests, or GitHub Enterprise later).
    pub fn with_endpoint(endpoint: String, token: String) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            // GitHub rejects requests without a User-Agent.
            .user_agent(concat!("isis/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| AppError::GitHub(format!("Could not create HTTP client: {err}")))?;
        Ok(Self {
            http,
            endpoint,
            token,
        })
    }

    /// The account the token belongs to.
    pub async fn viewer(&self) -> AppResult<GitHubUser> {
        #[derive(Deserialize)]
        struct Data {
            viewer: GitHubUser,
        }
        let data: Data = self
            .graphql("query { viewer { login name } }", json!({}))
            .await?;
        Ok(data.viewer)
    }

    /// Pull requests matching a GitHub search query (e.g. `is:pr author:@me`), all pages.
    pub async fn search_pull_requests(&self, query: &str) -> AppResult<Vec<PullRequest>> {
        #[derive(Deserialize)]
        struct Data {
            search: SearchConnection,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchConnection {
            page_info: PageInfo,
            #[serde(default)]
            nodes: Vec<Option<Value>>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PageInfo {
            has_next_page: bool,
            end_cursor: Option<String>,
        }

        let gql = format!(
            "query($q: String!, $cursor: String) {{
               search(query: $q, type: ISSUE, first: 100, after: $cursor) {{
                 pageInfo {{ hasNextPage endCursor }}
                 nodes {{ ... on PullRequest {{ {PR_FIELDS} }} }}
               }}
             }}"
        );
        let mut pull_requests = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_SEARCH_PAGES {
            let data: Data = self
                .graphql(&gql, json!({ "q": query, "cursor": cursor }))
                .await?;
            pull_requests.extend(parse_nodes(data.search.nodes));
            match data.search.page_info.end_cursor {
                Some(next) if data.search.page_info.has_next_page => cursor = Some(next),
                _ => break,
            }
        }
        Ok(pull_requests)
    }

    /// Looks pull requests up by node id. Ids that no longer resolve (deleted,
    /// no access) are skipped.
    pub async fn pull_requests_by_id(&self, ids: &[String]) -> AppResult<Vec<PullRequest>> {
        #[derive(Deserialize)]
        struct Data {
            #[serde(default)]
            nodes: Vec<Option<Value>>,
        }
        let gql = format!(
            "query($ids: [ID!]!) {{ nodes(ids: $ids) {{ ... on PullRequest {{ {PR_FIELDS} }} }} }}"
        );
        let mut pull_requests = Vec::new();
        for chunk in ids.chunks(NODES_CHUNK) {
            let data: Data = self.graphql(&gql, json!({ "ids": chunk })).await?;
            pull_requests.extend(parse_nodes(data.nodes));
        }
        Ok(pull_requests)
    }

    /// Runs a GraphQL query. Partial results (`data` plus `errors`, e.g. for ids that
    /// no longer exist) count as success; only a missing `data` is an error.
    async fn graphql<T: DeserializeOwned>(&self, query: &str, variables: Value) -> AppResult<T> {
        #[derive(Deserialize)]
        struct Response<T> {
            data: Option<T>,
            #[serde(default)]
            errors: Vec<GraphQlError>,
        }
        #[derive(Deserialize)]
        struct GraphQlError {
            message: String,
        }

        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|err| {
                AppError::GitHub(format!("Could not reach GitHub: {}", root_cause(&err)))
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::GitHub(match status {
                StatusCode::UNAUTHORIZED => "GitHub rejected the token".to_owned(),
                _ => match response.json::<Value>().await {
                    // e.g. rate limits: {"message": "API rate limit exceeded …"}
                    Ok(body) if body["message"].is_string() => {
                        format!("GitHub: {}", body["message"].as_str().unwrap_or_default())
                    }
                    _ => format!("GitHub responded with {status}"),
                },
            }));
        }

        let body: Response<T> = response
            .json()
            .await
            .map_err(|err| AppError::GitHub(format!("Unexpected response from GitHub: {err}")))?;
        body.data.ok_or_else(|| {
            let messages: Vec<_> = body.errors.into_iter().map(|e| e.message).collect();
            AppError::GitHub(if messages.is_empty() {
                "GitHub returned no data".to_owned()
            } else {
                format!("GitHub: {}", messages.join(" "))
            })
        })
    }
}

/// Keeps pull request nodes; search results can also contain issues (empty objects
/// here, since only the `PullRequest` fragment is selected) and `nodes` can contain nulls.
fn parse_nodes(nodes: Vec<Option<Value>>) -> impl Iterator<Item = PullRequest> {
    nodes
        .into_iter()
        .flatten()
        .filter_map(|node| serde_json::from_value::<PullRequestNode>(node).ok())
        .map(PullRequest::from)
}

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    id: String,
    number: i64,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    review_decision: Option<String>,
    updated_at: String,
    repository: Repository,
    author: Option<Actor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    name_with_owner: String,
}

#[derive(Deserialize)]
struct Actor {
    login: String,
}

impl From<PullRequestNode> for PullRequest {
    fn from(node: PullRequestNode) -> Self {
        Self {
            id: node.id,
            number: node.number,
            title: node.title,
            url: node.url,
            repository: node.repository.name_with_owner,
            author: node.author.map(|a| a.login).unwrap_or_default(),
            state: match node.state.as_str() {
                "MERGED" => PullRequestState::Merged,
                "CLOSED" => PullRequestState::Closed,
                _ => PullRequestState::Open,
            },
            is_draft: node.is_draft,
            review_decision: match node.review_decision.as_deref() {
                Some("APPROVED") => Some(ReviewDecision::Approved),
                Some("CHANGES_REQUESTED") => Some(ReviewDecision::ChangesRequested),
                Some("REVIEW_REQUIRED") => Some(ReviewDecision::ReviewRequired),
                _ => None,
            },
            updated_at: node.updated_at,
            // Local state, decided by the sync and workstream links.
            authored_by_me: false,
            review_requested: false,
            workstream_id: None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use mockito::Matcher;

    use super::*;

    pub fn client(server_url: &str) -> GitHubClient {
        GitHubClient::with_endpoint(format!("{server_url}/graphql"), "ghp_test".into()).unwrap()
    }

    /// A pull request node as GitHub returns it.
    pub fn pr_json(id: &str, number: i64, state: &str) -> String {
        format!(
            r#"{{"id":"{id}","number":{number},"title":"PR {number}","url":"https://github.com/acme/app/pull/{number}","state":"{state}","isDraft":false,"reviewDecision":"APPROVED","updatedAt":"2026-11-02T10:00:00Z","repository":{{"nameWithOwner":"acme/app"}},"author":{{"login":"ada"}}}}"#
        )
    }

    pub fn search_page(nodes: &[String], next: Option<&str>) -> String {
        let page_info = match next {
            Some(cursor) => format!(r#"{{"hasNextPage":true,"endCursor":"{cursor}"}}"#),
            None => r#"{"hasNextPage":false,"endCursor":null}"#.to_owned(),
        };
        format!(
            r#"{{"data":{{"search":{{"pageInfo":{page_info},"nodes":[{}]}}}}}}"#,
            nodes.join(",")
        )
    }

    #[tokio::test]
    async fn viewer_sends_bearer_token() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/graphql")
            .match_header("authorization", "Bearer ghp_test")
            .match_header("user-agent", Matcher::Regex("^isis/".into()))
            .with_body(r#"{"data":{"viewer":{"login":"ada","name":"Ada Lovelace"}}}"#)
            .create_async()
            .await;
        let user = client(&server.url()).viewer().await.unwrap();
        assert_eq!(user.login, "ada");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn errors_map_to_messages() {
        let mut server = mockito::Server::new_async().await;
        for (status, body, expected) in [
            (401, "", "GitHub rejected the token"),
            (
                403,
                r#"{"message":"API rate limit exceeded"}"#,
                "GitHub: API rate limit exceeded",
            ),
            (
                200,
                r#"{"errors":[{"message":"Something broke"}]}"#,
                "GitHub: Something broke",
            ),
            (502, "", "responded with 502"),
        ] {
            server.reset();
            server
                .mock("POST", "/graphql")
                .with_status(status)
                .with_body(body)
                .create_async()
                .await;
            match client(&server.url()).viewer().await {
                Err(AppError::GitHub(message)) => assert!(message.contains(expected), "{message}"),
                other => panic!("expected GitHub error for {status}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn search_follows_cursors_and_skips_non_pull_requests() {
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("POST", "/graphql")
            .match_body(Matcher::PartialJsonString(
                r#"{"variables":{"cursor":null}}"#.into(),
            ))
            .with_body(search_page(
                &[pr_json("PR_1", 1, "OPEN"), "{}".into()],
                Some("c2"),
            ))
            .create_async()
            .await;
        let second = server
            .mock("POST", "/graphql")
            .match_body(Matcher::PartialJsonString(
                r#"{"variables":{"cursor":"c2"}}"#.into(),
            ))
            .with_body(search_page(&[pr_json("PR_2", 2, "MERGED")], None))
            .create_async()
            .await;

        let prs = client(&server.url())
            .search_pull_requests("is:pr author:@me")
            .await
            .unwrap();
        assert_eq!(prs.len(), 2);
        assert_eq!(prs[0].repository, "acme/app");
        assert_eq!(prs[0].review_decision, Some(ReviewDecision::Approved));
        assert_eq!(prs[1].state, PullRequestState::Merged);
        first.assert_async().await;
        second.assert_async().await;
    }

    #[tokio::test]
    async fn by_id_tolerates_missing_nodes() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/graphql")
            .with_body(format!(
                r#"{{"data":{{"nodes":[{},null]}},"errors":[{{"type":"NOT_FOUND","message":"Could not resolve"}}]}}"#,
                pr_json("PR_1", 1, "CLOSED")
            ))
            .create_async()
            .await;
        let prs = client(&server.url())
            .pull_requests_by_id(&["PR_1".into(), "PR_gone".into()])
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].state, PullRequestState::Closed);
    }
}

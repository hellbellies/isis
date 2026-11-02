use std::fs;
use std::path::Path;
use std::time::Duration;

use reqwest::StatusCode;
use url::Url;

use crate::error::{AppError, AppResult};
use crate::models::jira::{JiraConfig, JiraSettings, JiraSettingsInput, JiraUser};

pub const JIRA_CONFIG_FILE: &str = "jira.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Storage for a single secret. Abstracted so tests don't touch the real keychain.
pub trait SecretStore {
    fn get(&self) -> AppResult<Option<String>>;
    fn set(&self, secret: &str) -> AppResult<()>;
    fn delete(&self) -> AppResult<()>;
}

/// OS keychain entry (macOS Keychain, Windows Credential Manager, Secret Service on Linux).
pub struct KeyringStore {
    service: &'static str,
    user: &'static str,
}

impl KeyringStore {
    pub const fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    fn entry(&self) -> AppResult<keyring::Entry> {
        Ok(keyring::Entry::new(self.service, self.user)?)
    }
}

impl SecretStore for KeyringStore {
    fn get(&self) -> AppResult<Option<String>> {
        match self.entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn set(&self, secret: &str) -> AppResult<()> {
        Ok(self.entry()?.set_password(secret)?)
    }

    fn delete(&self) -> AppResult<()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

fn load_config(dir: &Path) -> AppResult<JiraConfig> {
    let path = dir.join(JIRA_CONFIG_FILE);
    if !path.exists() {
        return Ok(JiraConfig::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn save_config(dir: &Path, config: &JiraConfig) -> AppResult<()> {
    fs::create_dir_all(dir)?;
    let raw = serde_json::to_string_pretty(config)?;
    fs::write(dir.join(JIRA_CONFIG_FILE), raw)?;
    Ok(())
}

/// Validates and normalizes a Jira base URL to `scheme://host[:port][/path]`
/// without a trailing slash. An empty input means "not configured".
fn normalize_base_url(input: &str) -> AppResult<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let invalid = || AppError::Validation(format!("'{trimmed}' is not a valid Jira URL"));
    let mut url = Url::parse(trimmed).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
        return Err(invalid());
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

/// Returns the stored connection details and whether a token is present.
pub fn get(dir: &Path, secrets: &impl SecretStore) -> AppResult<JiraSettings> {
    let config = load_config(dir)?;
    Ok(JiraSettings {
        base_url: config.base_url,
        email: config.email,
        has_token: secrets.get()?.is_some(),
    })
}

/// Saves URL and email to disk and, if provided, the API token to `secrets`.
pub fn save(
    dir: &Path,
    secrets: &impl SecretStore,
    input: JiraSettingsInput,
) -> AppResult<JiraSettings> {
    let config = JiraConfig {
        base_url: normalize_base_url(&input.base_url)?,
        email: input.email.trim().to_owned(),
    };
    let token = match input.api_token.as_deref().map(str::trim) {
        Some("") => return Err(AppError::Validation("API token cannot be empty".into())),
        other => other,
    };

    save_config(dir, &config)?;
    if let Some(token) = token {
        secrets.set(token)?;
    }
    get(dir, secrets)
}

/// Removes the API token from `secrets`, keeping URL and email.
pub fn clear_token(dir: &Path, secrets: &impl SecretStore) -> AppResult<JiraSettings> {
    secrets.delete()?;
    get(dir, secrets)
}

/// Checks the given settings against Jira without saving them.
/// A missing `api_token` falls back to the token stored in `secrets`.
pub async fn test_connection(
    secrets: &impl SecretStore,
    input: JiraSettingsInput,
) -> AppResult<JiraUser> {
    let base_url = normalize_base_url(&input.base_url)?;
    if base_url.is_empty() {
        return Err(AppError::Validation("Enter the Jira site URL".into()));
    }
    let email = input.email.trim();
    if email.is_empty() {
        return Err(AppError::Validation(
            "Enter the email of your Atlassian account".into(),
        ));
    }
    let token = match input.api_token.as_deref().map(str::trim) {
        Some(token) if !token.is_empty() => token.to_owned(),
        _ => secrets
            .get()?
            .ok_or_else(|| AppError::Validation("Enter an API token".into()))?,
    };
    check_connection(&base_url, email, &token).await
}

/// Calls `GET /rest/api/2/myself` (available on Cloud and Data Center)
/// and maps failures to messages a user can act on.
pub async fn check_connection(base_url: &str, email: &str, token: &str) -> AppResult<JiraUser> {
    let unreachable = |err: reqwest::Error| {
        AppError::Jira(format!("Could not reach {base_url}: {}", root_cause(&err)))
    };
    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(unreachable)?
        .get(format!("{base_url}/rest/api/2/myself"))
        .basic_auth(email, Some(token))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(unreachable)?;

    match response.status() {
        status if status.is_success() => response.json::<JiraUser>().await.map_err(|_| {
            AppError::Jira(format!("{base_url} did not respond like a Jira site"))
        }),
        StatusCode::UNAUTHORIZED => Err(AppError::Jira(
            "Jira rejected the email or API token".into(),
        )),
        StatusCode::FORBIDDEN => Err(AppError::Jira(
            "Access denied. Log in to Jira in the browser once (it may require a CAPTCHA), then retry".into(),
        )),
        StatusCode::NOT_FOUND => Err(AppError::Jira(format!(
            "No Jira REST API found at {base_url}"
        ))),
        status => Err(AppError::Jira(format!("Jira responded with {status}"))),
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct MemoryStore(RefCell<Option<String>>);

    impl SecretStore for MemoryStore {
        fn get(&self) -> AppResult<Option<String>> {
            Ok(self.0.borrow().clone())
        }
        fn set(&self, secret: &str) -> AppResult<()> {
            *self.0.borrow_mut() = Some(secret.to_owned());
            Ok(())
        }
        fn delete(&self) -> AppResult<()> {
            *self.0.borrow_mut() = None;
            Ok(())
        }
    }

    fn input(base_url: &str, api_token: Option<&str>) -> JiraSettingsInput {
        JiraSettingsInput {
            base_url: base_url.into(),
            email: " me@example.com ".into(),
            api_token: api_token.map(Into::into),
        }
    }

    #[test]
    fn get_returns_empty_when_unconfigured() {
        let dir = tempfile::tempdir().unwrap();
        let settings = get(dir.path(), &MemoryStore::default()).unwrap();
        assert_eq!(settings.base_url, "");
        assert!(!settings.has_token);
    }

    #[test]
    fn save_persists_config_and_token_separately() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        let settings = save(
            dir.path(),
            &secrets,
            input("https://acme.atlassian.net/", Some(" secret ")),
        )
        .unwrap();

        assert_eq!(settings.base_url, "https://acme.atlassian.net");
        assert_eq!(settings.email, "me@example.com");
        assert!(settings.has_token);
        assert_eq!(secrets.get().unwrap().as_deref(), Some("secret"));

        let on_disk = fs::read_to_string(dir.path().join(JIRA_CONFIG_FILE)).unwrap();
        assert!(!on_disk.contains("secret"));
    }

    #[test]
    fn save_without_token_keeps_existing_token() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        save(
            dir.path(),
            &secrets,
            input("https://a.atlassian.net", Some("t1")),
        )
        .unwrap();
        save(dir.path(), &secrets, input("https://b.atlassian.net", None)).unwrap();
        assert_eq!(secrets.get().unwrap().as_deref(), Some("t1"));
    }

    #[test]
    fn save_rejects_invalid_url_and_blank_token() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        for bad in ["acme.atlassian.net", "ftp://acme.net", "https://"] {
            assert!(matches!(
                save(dir.path(), &secrets, input(bad, None)),
                Err(AppError::Validation(_))
            ));
        }
        assert!(matches!(
            save(
                dir.path(),
                &secrets,
                input("https://a.atlassian.net", Some("  "))
            ),
            Err(AppError::Validation(_))
        ));
        assert!(!dir.path().join(JIRA_CONFIG_FILE).exists());
    }

    #[test]
    fn normalize_keeps_context_path_and_drops_query() {
        assert_eq!(
            normalize_base_url("https://jira.corp.local:8443/jira/?x=1#y").unwrap(),
            "https://jira.corp.local:8443/jira"
        );
    }

    #[test]
    fn clear_token_keeps_config() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        save(
            dir.path(),
            &secrets,
            input("https://a.atlassian.net", Some("t")),
        )
        .unwrap();
        let settings = clear_token(dir.path(), &secrets).unwrap();
        assert!(!settings.has_token);
        assert_eq!(settings.base_url, "https://a.atlassian.net");
    }

    const MYSELF: &str = "/rest/api/2/myself";
    const AUTH: &str = "Basic bWVAZXhhbXBsZS5jb206c2VjcmV0"; // me@example.com:secret

    #[tokio::test]
    async fn check_connection_returns_user_on_success() {
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

        let user = check_connection(&server.url(), "me@example.com", "secret")
            .await
            .unwrap();
        assert_eq!(user.display_name, "Ada Lovelace");
        assert_eq!(user.email_address.as_deref(), Some("me@example.com"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn check_connection_maps_failures_to_messages() {
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
        ] {
            server.reset();
            server
                .mock("GET", MYSELF)
                .with_status(status)
                .with_body(body)
                .create_async()
                .await;
            match check_connection(&server.url(), "me@example.com", "secret").await {
                Err(AppError::Jira(message)) => assert!(message.contains(expected), "{message}"),
                other => panic!("expected Jira error for {status}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn test_connection_falls_back_to_stored_token() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", MYSELF)
            .match_header("authorization", AUTH)
            .with_body(r#"{"displayName":"Ada"}"#)
            .create_async()
            .await;
        let secrets = MemoryStore::default();
        secrets.set("secret").unwrap();

        let user = test_connection(&secrets, input(&server.url(), None))
            .await
            .unwrap();
        assert_eq!(user.display_name, "Ada");
        assert_eq!(user.email_address, None);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_connection_requires_all_fields() {
        let secrets = MemoryStore::default();
        for bad in [
            input("", Some("secret")),
            JiraSettingsInput {
                email: " ".into(),
                ..input("https://a.atlassian.net", Some("secret"))
            },
            input("https://a.atlassian.net", None),
        ] {
            assert!(matches!(
                test_connection(&secrets, bad).await,
                Err(AppError::Validation(_))
            ));
        }
    }
}

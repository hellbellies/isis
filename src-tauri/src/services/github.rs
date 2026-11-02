//! GitHub settings: optional repository scope in `github.json`, token in the keychain.

use std::fs;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::models::github::{GitHubConfig, GitHubSettings, GitHubSettingsInput, GitHubUser};
use crate::services::github_client::GitHubClient;
use crate::services::secrets::SecretStore;

pub const GITHUB_CONFIG_FILE: &str = "github.json";

pub fn load_config(dir: &Path) -> AppResult<GitHubConfig> {
    let path = dir.join(GITHUB_CONFIG_FILE);
    if !path.exists() {
        return Ok(GitHubConfig::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn save_config(dir: &Path, config: &GitHubConfig) -> AppResult<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(GITHUB_CONFIG_FILE),
        serde_json::to_string_pretty(config)?,
    )?;
    Ok(())
}

pub fn get(dir: &Path, secrets: &impl SecretStore) -> AppResult<GitHubSettings> {
    Ok(GitHubSettings {
        repositories: load_config(dir)?.repositories,
        has_token: secrets.get()?.is_some(),
    })
}

/// Saves the repository scope and, if provided, the token.
pub fn save(
    dir: &Path,
    secrets: &impl SecretStore,
    input: GitHubSettingsInput,
) -> AppResult<GitHubSettings> {
    let config = GitHubConfig {
        repositories: normalize_repositories(&input.repositories)?,
    };
    let token = match input.token.as_deref().map(str::trim) {
        Some("") => return Err(AppError::Validation("The token cannot be empty".into())),
        other => other,
    };
    save_config(dir, &config)?;
    if let Some(token) = token {
        secrets.set(token)?;
    }
    get(dir, secrets)
}

pub fn clear_token(dir: &Path, secrets: &impl SecretStore) -> AppResult<GitHubSettings> {
    secrets.delete()?;
    get(dir, secrets)
}

/// The stored token, or a hint to configure one.
pub fn token(secrets: &impl SecretStore) -> AppResult<String> {
    secrets
        .get()?
        .ok_or_else(|| AppError::Validation("Add a GitHub token in the settings".into()))
}

/// Checks a token (the given one, or the stored one when `None`) against GitHub.
pub async fn test_connection(
    secrets: &impl SecretStore,
    token_input: Option<String>,
) -> AppResult<GitHubUser> {
    let token = match token_input.as_deref().map(str::trim) {
        Some(token) if !token.is_empty() => token.to_owned(),
        _ => self::token(secrets)?,
    };
    GitHubClient::new(token)?.viewer().await
}

/// Search qualifiers restricting results to the configured repositories/owners
/// (GitHub ORs multiple `repo:`/`user:` qualifiers). Empty for "everywhere".
pub fn search_scope(repositories: &[String]) -> String {
    repositories
        .iter()
        .map(|entry| match entry.contains('/') {
            true => format!("repo:{entry}"),
            false => format!("user:{entry}"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Trims entries, accepts pasted GitHub URLs, drops duplicates (ignoring case)
/// and rejects anything that isn't `owner` or `owner/repo`. Validation also keeps
/// entries safe to embed in a search query.
fn normalize_repositories(entries: &[String]) -> AppResult<Vec<String>> {
    let mut normalized: Vec<String> = Vec::new();
    for raw in entries {
        let entry = raw.trim();
        let entry = entry
            .strip_prefix("https://github.com/")
            .unwrap_or(entry)
            .trim_end_matches('/');
        let entry = entry.strip_suffix(".git").unwrap_or(entry);
        if entry.is_empty() || normalized.iter().any(|e| e.eq_ignore_ascii_case(entry)) {
            continue;
        }
        let valid = match entry.split_once('/') {
            Some((owner, repo)) => is_owner(owner) && is_repo(repo),
            None => is_owner(entry),
        };
        if !valid {
            return Err(AppError::Validation(format!(
                "'{raw}' is not a GitHub owner or owner/repository"
            )));
        }
        normalized.push(entry.to_owned());
    }
    Ok(normalized)
}

/// GitHub user/org names: 1–39 alphanumerics or hyphens, not starting with a hyphen.
fn is_owner(name: &str) -> bool {
    (1..=39).contains(&name.len())
        && !name.starts_with('-')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Repository names: 1–100 alphanumerics, `.`, `_` or `-`, excluding `.` and `..`.
fn is_repo(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::secrets::tests::MemoryStore;

    fn input(repositories: &[&str], token: Option<&str>) -> GitHubSettingsInput {
        GitHubSettingsInput {
            repositories: repositories.iter().map(|r| r.to_string()).collect(),
            token: token.map(Into::into),
        }
    }

    #[test]
    fn save_normalizes_repositories_and_stores_token_separately() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        let settings = save(
            dir.path(),
            &secrets,
            input(
                &[
                    " acme/app ",
                    "https://github.com/acme/api.git",
                    "ACME/APP",
                    "",
                    "octo-org",
                ],
                Some(" ghp_secret "),
            ),
        )
        .unwrap();
        assert_eq!(settings.repositories, ["acme/app", "acme/api", "octo-org"]);
        assert!(settings.has_token);
        assert_eq!(secrets.get().unwrap().as_deref(), Some("ghp_secret"));
        let on_disk = fs::read_to_string(dir.path().join(GITHUB_CONFIG_FILE)).unwrap();
        assert!(!on_disk.contains("ghp_secret"));
    }

    #[test]
    fn save_rejects_invalid_entries_and_blank_token() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = MemoryStore::default();
        for bad in [
            "-acme",
            "/app",
            "a/b/c",
            "acme/..",
            "acme app",
            "acme/app\" is:issue",
        ] {
            assert!(
                matches!(
                    save(dir.path(), &secrets, input(&[bad], None)),
                    Err(AppError::Validation(_))
                ),
                "{bad}"
            );
        }
        assert!(save(dir.path(), &secrets, input(&[], Some("  "))).is_err());
    }

    #[test]
    fn search_scope_maps_entries_to_qualifiers() {
        let repos = ["acme/app".to_string(), "octo-org".to_string()];
        assert_eq!(search_scope(&repos), "repo:acme/app user:octo-org");
        assert_eq!(search_scope(&[]), "");
    }

    #[test]
    fn token_requires_a_stored_token() {
        assert!(matches!(
            token(&MemoryStore::default()),
            Err(AppError::Validation(_))
        ));
    }
}

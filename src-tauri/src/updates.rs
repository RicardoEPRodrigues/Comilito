//! Checks whether a newer Comilito has been released (decision 0066), once per run, when the
//! preference allows. It asks GitHub's API for the newest release of the repository and
//! compares its version with this one. Nothing is downloaded or installed: the UI tells the
//! user, with a button to the website's downloads.

use std::time::Duration;

use serde::Serialize;

const LATEST_RELEASE: &str = "https://api.github.com/repos/RicardoEPRodrigues/Comilito/releases/latest";
/// Where the UI sends people to download it.
const DOWNLOAD_PAGE: &str = "https://comilito.colorcrow.me/#download";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    /// The newer version, like "1.1.0".
    pub version: String,
    pub download_page: String,
}

/// The newer release, if there is one. `Err` when GitHub can't be reached or answers
/// something unexpected; the UI stays quiet then (offline is normal).
#[tauri::command(async)]
pub fn check_for_update() -> Result<Option<Update>, String> {
    let latest = latest_version()?;
    let current = env!("CARGO_PKG_VERSION");
    Ok(is_newer(&latest, current).then(|| Update {
        version: latest,
        download_page: DOWNLOAD_PAGE.into(),
    }))
}

/// The newest published release's version (its tag without the `v`). Drafts and pre-releases
/// aren't "latest" on GitHub, so they're never offered.
fn latest_version() -> Result<String, String> {
    // A short timeout: this runs in the background, and a slow network mustn't keep a thread
    // waiting for minutes.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let body = agent
        .get(LATEST_RELEASE)
        // GitHub's API refuses requests without a User-Agent.
        .header("User-Agent", concat!("Comilito/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("couldn't check for a new version: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("couldn't check for a new version: {e}"))?;
    tag_version(&body).ok_or_else(|| "couldn't check for a new version: unexpected answer from GitHub".into())
}

/// The version in a release's JSON: `tag_name` "v1.1.0" → "1.1.0".
fn tag_version(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    let version = tag.strip_prefix('v').unwrap_or(tag);
    parse(version).map(|_| version.to_string())
}

/// "1.2.3" → [1, 2, 3]. Anything else (a pre-release suffix included) is `None`.
fn parse(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.').map(|p| p.parse::<u64>().ok());
    let v = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(v)
}

/// Whether version `a` comes after `b`. Arrays compare element by element, like versions.
fn is_newer(a: &str, b: &str) -> bool {
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.0", "1.0.0"));
        assert!(!is_newer("1.1.0-beta", "1.0.0"));
    }

    #[test]
    fn reads_the_tag() {
        assert_eq!(tag_version(r#"{"tag_name":"v1.2.3","name":"x"}"#).as_deref(), Some("1.2.3"));
        assert_eq!(tag_version(r#"{"tag_name":"1.2.3"}"#).as_deref(), Some("1.2.3"));
        assert_eq!(tag_version(r#"{"tag_name":"nightly"}"#), None);
        assert_eq!(tag_version(r#"{"message":"Not Found"}"#), None);
    }
}

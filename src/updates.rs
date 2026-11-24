//! GitLab Releases update check (no tokens stored or required).
use serde::Deserialize;

pub const DEFAULT_PROJECT: &str = "aurafy-project1/aurafy-downloads";
pub const RELEASES_URL: &str = "https://gitlab.com/aurafy-project1/aurafy-downloads/-/releases";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub name: String,
    pub url: String,
    pub notes: String,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    UpToDate(ReleaseInfo),
    UpdateAvailable(ReleaseInfo),
    NoReleases,
    PrivateOrUnavailable(String),
}

#[derive(Default)]
pub struct State {
    pub checking: bool,
    pub outcome: Option<CheckOutcome>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct GitLabRelease {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    released_at: Option<String>,
    #[serde(rename = "_links", default)]
    links: Option<GitLabLinks>,
}

#[derive(Deserialize)]
struct GitLabLinks {
    #[serde(rename = "self", default)]
    self_url: String,
}

pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next().unwrap_or(trimmed);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer(candidate_tag: &str, current_version: &str) -> bool {
    match (parse_semver(candidate_tag), parse_semver(current_version)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

pub fn parse_gitlab_releases(body: &str, current_version: &str) -> Result<CheckOutcome, String> {
    let releases: Vec<GitLabRelease> = serde_json::from_str(body)
        .map_err(|error| format!("Could not read GitLab releases response: {error}"))?;
    let Some(best) = releases
        .into_iter()
        .max_by_key(|release| parse_semver(&release.tag_name).unwrap_or((0, 0, 0)))
    else {
        return Ok(CheckOutcome::NoReleases);
    };
    let url = best
        .links
        .map(|links| links.self_url)
        .filter(|url| {
            url::Url::parse(url).is_ok_and(|parsed| {
                parsed.scheme() == "https"
                    && parsed.host_str() == Some("gitlab.com")
                    && parsed.username().is_empty()
                    && parsed.password().is_none()
            })
        })
        .unwrap_or_else(|| RELEASES_URL.to_string());
    let name = if best.name.as_deref().unwrap_or_default().trim().is_empty() {
        best.tag_name.clone()
    } else {
        best.name.as_deref().unwrap_or_default().trim().to_string()
    };
    let info = ReleaseInfo {
        tag: best.tag_name.trim().to_string(),
        name,
        url,
        notes: best
            .description
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
        published_at: best.released_at,
    };
    if is_newer(&info.tag, current_version) {
        Ok(CheckOutcome::UpdateAvailable(info))
    } else {
        Ok(CheckOutcome::UpToDate(info))
    }
}

pub fn releases_api_url(project: &str) -> String {
    let encoded = project.trim_matches('/').replace('/', "%2F");
    format!("https://gitlab.com/api/v4/projects/{encoded}/releases?per_page=10")
}

pub fn check_latest(current_version: &str) -> Result<CheckOutcome, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("Aurafy/{current_version}"))
        .build()
        .map_err(|error| format!("Could not start update client: {error}"))?;
    let response = client
        .get(releases_api_url(DEFAULT_PROJECT))
        .send()
        .map_err(|error| format!("Could not reach GitLab Releases: {error}"))?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 || status.as_u16() == 404 {
        return Ok(CheckOutcome::PrivateOrUnavailable(
            "GitLab releases repository is private or has no public releases yet. Open the releases page in your browser if you have access.".into(),
        ));
    }
    if !status.is_success() {
        return Err(format!("GitLab returned status {status}."));
    }
    let body = response
        .text()
        .map_err(|error| format!("Could not read GitLab releases response: {error}"))?;
    parse_gitlab_releases(&body, current_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_handles_tags_and_updates() {
        assert_eq!(parse_semver("v0.8.1"), Some((0, 8, 1)));
        assert_eq!(parse_semver("0.9.0-rc.1"), Some((0, 9, 0)));
        assert!(is_newer("v0.8.1", "0.8.0"));
        assert!(is_newer("v0.9.0", "0.8.0"));
        assert!(is_newer("1.0.0", "0.8.0"));
        assert!(!is_newer("v0.8.0", "0.8.0"));
        assert!(!is_newer("v0.7.9", "0.8.0"));
    }

    #[test]
    fn gitlab_releases_json_detects_newer_and_current_versions() {
        let payload = r#"[
            {
                "tag_name": "v0.8.0",
                "name": "Aurafy 0.8.0",
                "description": "Current release",
                "_links": {"self": "https://gitlab.com/aurafy-project1/aurafy-downloads/-/releases/v0.8.0"}
            },
            {
                "tag_name": "v0.8.1",
                "name": "Aurafy 0.8.1",
                "description": "Windows SMTC & i18n",
                "_links": {"self": "https://gitlab.com/aurafy-project1/aurafy-downloads/-/releases/v0.8.1"}
            }
        ]"#;
        let newer = parse_gitlab_releases(payload, "0.8.0").unwrap();
        match newer {
            CheckOutcome::UpdateAvailable(info) => {
                assert_eq!(info.tag, "v0.8.1");
                assert_eq!(info.name, "Aurafy 0.8.1");
                assert!(info.url.ends_with("v0.8.1"));
            }
            other => panic!("expected update available, got {other:?}"),
        }

        let current = parse_gitlab_releases(payload, "0.8.1").unwrap();
        assert!(matches!(current, CheckOutcome::UpToDate(_)));

        let empty = parse_gitlab_releases("[]", "0.8.0").unwrap();
        assert_eq!(empty, CheckOutcome::NoReleases);
    }
}

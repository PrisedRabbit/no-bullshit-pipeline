//! Lightweight update check against GitHub Releases.
//!
//! On startup the frontend invokes `check_for_updates`; if the latest
//! published release is newer than `CARGO_PKG_VERSION` we return enough info
//! to render a "v0.x.y available — Download" banner that links to the
//! release page. The user downloads + installs the DMG manually — there is
//! no auto-install (would require signed update artifacts).

use serde::{Deserialize, Serialize};

const RELEASES_API: &str =
    "https://api.github.com/repos/skopanev/no-bullshit-pipeline/releases/latest";

/// Where the Download button takes the user. Points at the releases page
/// (not the specific tag) so the user sees the full changelog and can pick
/// the right asset.
const RELEASES_PAGE: &str =
    "https://github.com/skopanev/no-bullshit-pipeline/releases";

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub name: Option<String>,
    pub current: String,
}

/// Parse `vMAJOR.MINOR.PATCH` (or without the leading `v`) into a tuple.
/// Anything beyond patch (rc/beta suffixes) is ignored on purpose — we
/// only ship plain semver tags from build.sh.
fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let mut it = s.trim_start_matches('v').split('.');
    let major = it.next()?.parse::<u32>().ok()?;
    let minor = it.next()?.parse::<u32>().ok()?;
    let patch = it
        .next()
        .and_then(|p| {
            // Strip pre-release / build metadata so "0.4.53-rc1" still
            // parses as 53. parse only the leading digits.
            let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u32>().ok()
        })
        .unwrap_or(0);
    Some((major, minor, patch))
}

/// Check GitHub Releases for a newer version. Returns `None` when the
/// running build is already current OR when the network call failed for
/// any reason (offline, GitHub rate limit, parse error) — we never want a
/// missing internet connection to surface a scary error in the UI.
#[tauri::command]
pub async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    let current_raw = env!("CARGO_PKG_VERSION");
    let Some(current_v) = parse_semver(current_raw) else {
        return Ok(None);
    };

    // GitHub requires a User-Agent on every request; the rustls reqwest in
    // this crate handles TLS.
    let client = match reqwest::Client::builder()
        .user_agent(concat!("nbp/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let resp = match client.get(RELEASES_API).send().await {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    if !resp.status().is_success() {
        return Ok(None);
    }
    let rel: GhRelease = match resp.json().await {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let Some(latest_v) = parse_semver(&rel.tag_name) else {
        return Ok(None);
    };

    if latest_v <= current_v {
        return Ok(None);
    }
    let _ = rel.html_url; // not used — we open the releases page instead
    Ok(Some(UpdateInfo {
        version: rel.tag_name,
        url: RELEASES_PAGE.to_string(),
        name: rel.name,
        current: current_raw.to_string(),
    }))
}

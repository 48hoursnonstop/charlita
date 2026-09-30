use crate::engine::Engine;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{path::PathBuf, sync::RwLock};

#[derive(Clone, Default)]
pub struct UpdateState {
    pub status: String,
    pub release: Option<Release>,
    pub downloaded: Option<PathBuf>,
}
#[derive(Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub prerelease: bool,
    pub draft: bool,
}
#[derive(Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}
fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Charlita/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .build()?)
}
fn status(e: &Engine, text: &str) {
    e.updates.write().unwrap().status = text.into();
    e.notify();
}
pub fn valid_repo(repo: &str) -> bool {
    let parts: Vec<_> = repo.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
        && !parts.iter().any(|p| *p == "." || *p == "..")
}
pub async fn check(e: Engine) -> Result<()> {
    let repo = e.live.read().unwrap().settings.update_repo.clone();
    ensure!(
        valid_repo(&repo),
        "Use owner/repo for the GitHub repository / Usa propietario/repositorio"
    );
    status(&e, "checking");
    let response = client()?
        .get(format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        status(&e, "no_releases");
        return Ok(());
    }
    ensure!(
        response.status().is_success(),
        "GitHub could not check updates; try again later"
    );
    let release: Release = response.json().await?;
    ensure!(
        !release.draft && !release.prerelease,
        "Only stable published releases are accepted"
    );
    let remote = semver::Version::parse(release.tag_name.trim_start_matches('v'))?;
    let local = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
    *e.updates.write().unwrap() = UpdateState {
        status: if remote > local {
            "available"
        } else {
            "current"
        }
        .into(),
        release: Some(release),
        downloaded: None,
    };
    e.notify();
    Ok(())
}
fn selected_asset(r: &Release, portable: bool) -> Option<&ReleaseAsset> {
    let suffix = match (cfg!(windows), portable) {
        (true, false) => "windows-x86_64-setup.exe",
        (true, true) => "windows-x86_64-portable.zip",
        (false, false) => "linux-x86_64.AppImage",
        (false, true) => "linux-x86_64-portable.tar.gz",
    };
    r.assets.iter().find(|a| a.name.ends_with(suffix))
}
pub async fn install(e: Engine) -> Result<()> {
    let release = e
        .updates
        .read()
        .unwrap()
        .release
        .clone()
        .context("Check updates first")?;
    let portable = e.store.metadata_value("portable_distribution")?.as_deref() == Some("true");
    let asset = selected_asset(&release, portable)
        .context("No compatible installer in this release")?
        .clone();
    let checksums = release
        .assets
        .iter()
        .find(|a| a.name == "SHA256SUMS")
        .context("Release has no SHA256SUMS; download from GitHub instead")?;
    ensure!(
        asset.size <= 1024 * 1024 * 1024,
        "Installer larger than 1 GiB"
    );
    for url in [&asset.browser_download_url, &checksums.browser_download_url] {
        let u = reqwest::Url::parse(url)?;
        ensure!(
            u.scheme() == "https" && u.host_str() == Some("github.com"),
            "Unexpected release download host"
        );
    }
    status(&e, "downloading");
    let http = client()?;
    let sums = http
        .get(&checksums.browser_download_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let expected = sums
        .lines()
        .find_map(|l| {
            let (hash, name) = l.split_once(char::is_whitespace)?;
            if name.trim().trim_start_matches('*') == asset.name {
                Some(hash.to_owned())
            } else {
                None
            }
        })
        .context("Installer is not listed in SHA256SUMS")?;
    let dir = e.store.root.join("updates");
    tokio::fs::create_dir_all(&dir).await?;
    let dest = dir.join(&asset.name);
    ensure!(
        std::path::Path::new(&asset.name)
            .file_name()
            .and_then(|s| s.to_str())
            == Some(asset.name.as_str()),
        "Invalid installer filename"
    );
    let temp = dest.with_extension("download");
    let mut file = tokio::fs::File::create(&temp).await?;
    let mut response = http
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?;
    let mut size = 0;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = response.chunk().await? {
        size += chunk.len() as u64;
        ensure!(
            size <= asset.size && size <= 1024 * 1024 * 1024,
            "Unexpected installer size"
        );
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    drop(file);
    ensure!(size == asset.size, "Incomplete installer download");
    ensure!(
        crate::assets::hash_file(&temp)? == expected,
        "Installer checksum failed. Try downloading again from GitHub"
    );
    tokio::fs::rename(temp, &dest).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).await?;
    }
    e.updates.write().unwrap().downloaded = Some(dest);
    status(&e, "ready");
    Ok(())
}
pub fn record_error(state: &RwLock<UpdateState>, err: &anyhow::Error) {
    state.write().unwrap().status = format!("error: {err}");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_validation() {
        assert!(valid_repo("48hoursnonstop/charlita"));
        for bad in ["https://github.com/a/b", "a/../b", "a/", "../b", "a/b?x=1"] {
            assert!(!valid_repo(bad))
        }
    }
}

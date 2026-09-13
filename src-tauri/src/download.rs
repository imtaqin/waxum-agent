//! Auto-downloads the waxum binary for "bundled" mode, so the user never
//! has to go find and unpack a release archive themselves. Fetches the
//! latest GitHub release, grabs the asset matching the running OS/arch,
//! unpacks it into the app's data dir, and returns the binary path —
//! cached there for next launch, only re-downloaded if missing.

use std::path::PathBuf;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, AppResult};

const REPO: &str = "imtaqin/waxum";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "waxum.exe"
    } else {
        "waxum"
    }
}

fn asset_suffix() -> AppResult<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-amd64.tar.gz"),
        ("linux", "aarch64") => Ok("linux-arm64.tar.gz"),
        ("windows", "x86_64") => Ok("windows-amd64.zip"),
        (os, arch) => Err(AppError::Bundled(format!(
            "no prebuilt waxum release for {os}/{arch} — use remote mode, or set the binary path manually to something you built yourself"
        ))),
    }
}

fn install_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Bundled(format!("resolving app data dir: {e}")))?
        .join("waxum-bin");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Returns the path to a local waxum binary, downloading the latest
/// release if one isn't already cached in the app data dir.
pub async fn ensure_binary(app: &AppHandle) -> AppResult<String> {
    let dir = install_dir(app)?;
    let target = dir.join(binary_name());
    if target.is_file() {
        return Ok(target.to_string_lossy().into_owned());
    }
    download_latest(app, &dir, &target).await?;
    Ok(target.to_string_lossy().into_owned())
}

/// Forces a fresh download even if a binary is already cached — used by
/// the settings screen's "Update bundled binary" action.
pub async fn download_latest_forced(app: &AppHandle) -> AppResult<String> {
    let dir = install_dir(app)?;
    let target = dir.join(binary_name());
    download_latest(app, &dir, &target).await?;
    Ok(target.to_string_lossy().into_owned())
}

async fn download_latest(app: &AppHandle, dir: &std::path::Path, target: &std::path::Path) -> AppResult<()> {
    let suffix = asset_suffix()?;
    let client = reqwest::Client::builder()
        .user_agent("waxum-agent")
        .build()?;

    let release: Release = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AppError::Bundled(format!("fetching latest release: {e}")))?
        .json()
        .await?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(suffix))
        .ok_or_else(|| {
            AppError::Bundled(format!(
                "release {} has no asset ending in {suffix}",
                release.tag_name
            ))
        })?;

    let _ = app.emit(
        "waxum-agent://download-progress",
        format!("downloading waxum {} ({suffix})", release.tag_name),
    );

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AppError::Bundled(format!("downloading asset: {e}")))?
        .bytes()
        .await?;

    if suffix.ends_with(".tar.gz") {
        extract_tar_gz(&bytes, dir)?;
    } else {
        extract_zip(&bytes, dir)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(target)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(target, perms)?;
    }

    if !target.is_file() {
        return Err(AppError::Bundled(format!(
            "extracted archive but {} was not produced",
            target.display()
        )));
    }
    Ok(())
}

fn extract_tar_gz(bytes: &[u8], dest: &std::path::Path) -> AppResult<()> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|e| AppError::Bundled(format!("extracting tar.gz: {e}")))
}

fn extract_zip(bytes: &[u8], dest: &std::path::Path) -> AppResult<()> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| AppError::Bundled(format!("opening zip: {e}")))?;
    archive
        .extract(dest)
        .map_err(|e| AppError::Bundled(format!("extracting zip: {e}")))
}

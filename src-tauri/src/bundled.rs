//! Runs waxum inside the app as a child process ("bundled" mode), so a
//! desktop user never has to download, configure or start a gateway
//! themselves. Desktop only: there is no child-process launch on
//! Android/iOS, so the frontend offers only remote mode there.
//!
//! [`launch`] does everything the user would otherwise do by hand:
//!
//! - downloads the waxum binary for this OS on first use
//!   ([`crate::download::ensure_binary`]);
//! - generates a random superadmin token and JWT secret once and keeps
//!   them in the app data dir, so the gateway and the app always agree on
//!   credentials and nothing has to be typed in;
//! - starts waxum with its working directory in the app data dir, so its
//!   SQLite database and WhatsApp session keys land somewhere writable
//!   (not next to the installed .exe) and survive restarts;
//! - sends waxum's output to `waxum.log` there instead of a pipe nobody
//!   reads, since a full pipe buffer would block waxum mid-write;
//! - picks a free port if the default one is taken, and waits for
//!   `/livez` before returning, instead of sleeping a fixed time.
//!
//! The child is killed when the app exits (see `lib.rs`), and
//! `kill_on_drop` covers the paths where that handler doesn't run.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::process::Child;

use crate::error::{AppError, AppResult};

const DEFAULT_PORT: u16 = 3451;
const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Where the frontend reaches the bundled gateway.
#[derive(Clone, Debug, Serialize)]
pub struct LaunchInfo {
    pub base_url: String,
    pub token: String,
    pub binary_path: String,
    pub port: u16,
}

/// The running child plus what it was started with.
pub struct BundledProcess {
    child: Child,
    info: LaunchInfo,
}

fn data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Bundled(format!("resolving app data dir: {e}")))?
        .join("waxum-data");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn random_hex(bytes: usize) -> AppResult<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf)
        .map_err(|e| AppError::Bundled(format!("generating a token: {e}")))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// Reads a secret persisted in `dir/name`, creating it on first use.
fn persisted_secret(dir: &Path, name: &str) -> AppResult<String> {
    let path = dir.join(name);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if existing.len() >= 32 {
            return Ok(existing.to_string());
        }
    }
    let secret = random_hex(32)?;
    std::fs::write(&path, &secret)?;
    Ok(secret)
}

/// The default port when it's free, otherwise any free port.
fn pick_port() -> AppResult<u16> {
    if std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PORT)).is_ok() {
        return Ok(DEFAULT_PORT);
    }
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

async fn wait_until_ready(port: u16, child: &mut Child, log_path: &Path) -> AppResult<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let url = format!("http://127.0.0.1:{port}/livez");
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(AppError::Bundled(format!(
                "waxum exited during startup ({status}); see {}",
                log_path.display()
            )));
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::Bundled(format!(
                "waxum did not become ready within {}s; see {}",
                READY_TIMEOUT.as_secs(),
                log_path.display()
            )));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Makes sure the bundled gateway is running and returns how to reach it.
/// Idempotent: a second call while the child is alive returns the same
/// info without starting another one.
pub async fn launch(
    app: &AppHandle,
    state: &tokio::sync::Mutex<Option<BundledProcess>>,
    binary_override: Option<String>,
) -> AppResult<LaunchInfo> {
    let mut guard = state.lock().await;
    if let Some(proc) = guard.as_mut() {
        if proc.child.try_wait().ok().flatten().is_none() {
            return Ok(proc.info.clone());
        }
    }

    let binary_path = match binary_override.filter(|p| !p.trim().is_empty()) {
        Some(path) => path,
        None => crate::download::ensure_binary(app).await?,
    };
    let dir = data_dir(app)?;
    let token = persisted_secret(&dir, "superadmin_token")?;
    let jwt_secret = persisted_secret(&dir, "jwt_secret")?;
    let port = pick_port()?;

    let log_path = dir.join("waxum.log");
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let mut cmd = tokio::process::Command::new(&binary_path);
    cmd.current_dir(&dir)
        .env("PORT", port.to_string())
        .env("SUPERADMIN_TOKEN", &token)
        .env("JWT_SECRET", &jwt_secret)
        .env("DATABASE_URL", "sqlite://waxum.db")
        .env("WHATSAPP_STORAGE_PATH", "./whatsapp_sessions")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Bundled(format!("failed to start {binary_path}: {e}")))?;

    if let Err(e) = wait_until_ready(port, &mut child, &log_path).await {
        let _ = child.kill().await;
        return Err(e);
    }

    let info = LaunchInfo {
        base_url: format!("http://127.0.0.1:{port}/api/v1"),
        token,
        binary_path,
        port,
    };
    *guard = Some(BundledProcess {
        child,
        info: info.clone(),
    });
    Ok(info)
}

pub async fn stop(state: &tokio::sync::Mutex<Option<BundledProcess>>) -> AppResult<()> {
    let mut guard = state.lock().await;
    if let Some(mut proc) = guard.take() {
        let _ = proc.child.kill().await;
    }
    Ok(())
}

pub async fn is_running(state: &tokio::sync::Mutex<Option<BundledProcess>>) -> bool {
    let mut guard = state.lock().await;
    match guard.as_mut() {
        Some(proc) => proc.child.try_wait().ok().flatten().is_none(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_generated_once_and_then_reused() {
        let dir = std::env::temp_dir().join(format!("waxum-agent-test-{}", random_hex(4).unwrap()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = persisted_secret(&dir, "superadmin_token").unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(persisted_secret(&dir, "superadmin_token").unwrap(), first);
        assert_ne!(persisted_secret(&dir, "jwt_secret").unwrap(), first);
        std::fs::write(dir.join("superadmin_token"), "short").unwrap();
        assert_ne!(persisted_secret(&dir, "superadmin_token").unwrap(), "short");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_taken_default_port_falls_back_to_a_free_one() {
        let _hold = std::net::TcpListener::bind(("127.0.0.1", DEFAULT_PORT));
        let port = pick_port().unwrap();
        assert_ne!(port, 0);
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}

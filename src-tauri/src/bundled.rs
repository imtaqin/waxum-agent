//! Runs a local waxum binary as a child process, for the "bundled binary"
//! mode (as opposed to pointing the app at a remote waxum URL). Desktop
//! only — there is no child-process launch on Android/iOS, so the frontend
//! hides this mode on mobile.

use std::process::Stdio;

use tokio::process::Child;

use crate::error::{AppError, AppResult};

pub async fn start(
    binary_path: &str,
    port: u16,
    env: Vec<(String, String)>,
    state: &tokio::sync::Mutex<Option<Child>>,
) -> AppResult<()> {
    let mut guard = state.lock().await;
    if let Some(child) = guard.as_mut() {
        if child.try_wait().ok().flatten().is_none() {
            return Err(AppError::Bundled("already running".into()));
        }
    }

    let mut cmd = tokio::process::Command::new(binary_path);
    cmd.env("PORT", port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }

    let child = cmd
        .spawn()
        .map_err(|e| AppError::Bundled(format!("failed to start {binary_path}: {e}")))?;
    *guard = Some(child);
    Ok(())
}

pub async fn stop(state: &tokio::sync::Mutex<Option<Child>>) -> AppResult<()> {
    let mut guard = state.lock().await;
    if let Some(mut child) = guard.take() {
        let _ = child.kill().await;
    }
    Ok(())
}

pub async fn is_running(state: &tokio::sync::Mutex<Option<Child>>) -> bool {
    let mut guard = state.lock().await;
    match guard.as_mut() {
        Some(child) => child.try_wait().ok().flatten().is_none(),
        None => false,
    }
}

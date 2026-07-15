use super::CaptureError;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RETENTION_HOURS: u64 = 24;

pub fn captures_dir() -> Result<PathBuf, CaptureError> {
    let home = dirs::home_dir().ok_or_else(|| CaptureError::OutputWriteFailed {
        detail: "home directory unresolvable (HOME unset?)".to_string(),
    })?;
    let dir = home.join(".bpmux").join("captures");
    std::fs::create_dir_all(&dir).map_err(|e| CaptureError::OutputWriteFailed {
        detail: format!("create captures dir {}: {e}", dir.display()),
    })?;
    Ok(dir)
}

pub fn build_capture_filename(role: &str, now: chrono::DateTime<chrono::Utc>) -> String {
    let slug: String = role
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let trimmed = slug.trim_matches('_').to_string();
    let role_part = if trimmed.is_empty() {
        "agent".to_string()
    } else {
        trimmed
    };
    format!("{}-{}.png", now.format("%Y%m%dT%H%M%SZ"), role_part)
}

pub fn cleanup_old_captures(dir: &Path) -> usize {
    let cutoff = match SystemTime::now().checked_sub(Duration::from_secs(RETENTION_HOURS * 3600)) {
        Some(t) => t,
        None => return 0,
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_png = path.extension().and_then(|s| s.to_str()) == Some("png");
        if !is_png {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = match metadata.modified() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if modified < cutoff && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

pub async fn capture_window_to_path(
    window_id: u32,
    output_path: &Path,
) -> Result<(), CaptureError> {
    let path_arg = output_path.to_string_lossy().into_owned();
    let output = tokio::process::Command::new("screencapture")
        .arg("-l")
        .arg(window_id.to_string())
        .arg("-o")
        .arg("-x")
        .arg("-t")
        .arg("png")
        .arg(&path_arg)
        .output()
        .await
        .map_err(|e| CaptureError::CaptureFailed {
            detail: format!("spawn screencapture: {e}"),
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() || !output_path.exists() {
        return Err(classify_screencapture_failure(
            &stderr,
            output.status.code(),
        ));
    }
    Ok(())
}

pub fn classify_screencapture_failure(stderr: &str, code: Option<i32>) -> CaptureError {
    let needle = stderr.to_lowercase();
    if needle.contains("could not create image from window")
        || needle.contains("not authorized")
        || needle.contains("permission")
    {
        CaptureError::PermissionDenied {
            detail: format!(
                "screencapture failed: {stderr_trim}. Grant Screen Recording permission \
                to the terminal/process running bpmux: \
                System Settings → Privacy & Security → Screen Recording.",
                stderr_trim = stderr.trim()
            ),
        }
    } else {
        CaptureError::CaptureFailed {
            detail: format!("screencapture exit {:?} stderr={:?}", code, stderr.trim()),
        }
    }
}

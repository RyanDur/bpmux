use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RETENTION_HOURS: u64 = 24;

const WINDOW_DISCOVERY_SCRIPT: &str = r#"
import Quartz
let info = (CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]]) ?? []
var electronSeen = false
var candidates: [(wid: Int, width: Int, height: Int)] = []
for w in info {
    let owner = (w[kCGWindowOwnerName as String] as? String) ?? ""
    let title = (w[kCGWindowName as String] as? String) ?? ""
    if owner == "Electron" {
        electronSeen = true
        let wid = (w[kCGWindowNumber as String] as? Int) ?? -1
        let bounds = (w[kCGWindowBounds as String] as? [String: CGFloat]) ?? [:]
        let width = Int(bounds["Width"] ?? 0)
        let height = Int(bounds["Height"] ?? 0)
        if title == "bpmux" {
            print("\(wid) \(width) \(height)")
            exit(0)
        }
        if width >= 400 && height >= 300 {
            candidates.append((wid, width, height))
        }
    }
}
if let best = candidates.max(by: { ($0.width * $0.height) < ($1.width * $1.height) }) {
    print("\(best.wid) \(best.width) \(best.height)")
    exit(0)
}
print(electronSeen ? "NO_WINDOW" : "NO_ELECTRON")
exit(1)
"#;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureError {
    ElectronNotRunning,
    NoWindow,
    WindowMinimized,
    CaptureFailed { detail: String },
    OutputWriteFailed { detail: String },
    SwiftUnavailable { detail: String },
    PermissionDenied { detail: String },
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CaptureResult {
    pub path: String,
    pub window_id: u32,
    pub captured_at: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveredWindow {
    pub window_id: u32,
    pub width: u32,
    pub height: u32,
}

pub async fn find_bpmux_window() -> Result<DiscoveredWindow, CaptureError> {
    let output = match tokio::process::Command::new("swift")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return Err(CaptureError::SwiftUnavailable {
                detail: format!(
                    "spawn swift: {e}. Install Xcode Command Line Tools (`xcode-select --install`)"
                ),
            });
        }
    };
    let mut child = output;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| CaptureError::CaptureFailed {
            detail: "swift child has no stdin handle".to_string(),
        })?;
    let script = WINDOW_DISCOVERY_SCRIPT.to_string();
    let stdin_writer = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let mut stdin = stdin;
        stdin
            .write_all(script.as_bytes())
            .await
            .map_err(|e| format!("swift stdin write: {e}"))?;
        stdin
            .shutdown()
            .await
            .map_err(|e| format!("swift stdin shutdown: {e}"))?;
        Ok::<(), String>(())
    });
    let result = child
        .wait_with_output()
        .await
        .map_err(|e| CaptureError::CaptureFailed {
            detail: format!("wait swift: {e}"),
        })?;
    match stdin_writer.await {
        Ok(Ok(())) => {}
        Ok(Err(detail)) => {
            return Err(CaptureError::CaptureFailed { detail });
        }
        Err(join_err) => {
            return Err(CaptureError::CaptureFailed {
                detail: format!("swift stdin task panicked: {join_err}"),
            });
        }
    }
    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
    parse_window_discovery_output(&stdout)
}

pub fn parse_window_discovery_output(stdout: &str) -> Result<DiscoveredWindow, CaptureError> {
    let line = stdout.lines().last().unwrap_or("").trim();
    if line == "NO_ELECTRON" {
        return Err(CaptureError::ElectronNotRunning);
    }
    if line == "NO_WINDOW" {
        return Err(CaptureError::NoWindow);
    }
    let mut parts = line.split_whitespace();
    let wid_str = parts.next().unwrap_or("");
    let width_str = parts.next().unwrap_or("0");
    let height_str = parts.next().unwrap_or("0");
    let wid: i64 = wid_str.parse().map_err(|_| CaptureError::CaptureFailed {
        detail: format!("could not parse swift output: '{stdout}'"),
    })?;
    if wid < 0 {
        return Err(CaptureError::NoWindow);
    }
    let width: u32 = width_str.parse().unwrap_or(0);
    let height: u32 = height_str.parse().unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(CaptureError::WindowMinimized);
    }
    Ok(DiscoveredWindow {
        window_id: wid as u32,
        width,
        height,
    })
}

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn build_capture_filename_uses_iso_compact_timestamp_and_role_slug() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
        let name = build_capture_filename("dev", now);
        assert_eq!(name, "20260505T013045Z-dev.png");
    }

    #[test]
    fn build_capture_filename_slugs_unsafe_chars() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
        let name = build_capture_filename("agent/with spaces", now);
        assert_eq!(name, "20260505T013045Z-agent_with_spaces.png");
    }

    #[test]
    fn build_capture_filename_falls_back_to_agent_when_role_is_only_special_chars() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 5, 5, 1, 30, 45).unwrap();
        let name = build_capture_filename("///", now);
        assert!(
            name.contains("-agent.png"),
            "all-special role slugs must fall back to 'agent' to avoid trailing dash"
        );
    }

    #[test]
    fn cleanup_old_captures_returns_zero_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let removed = cleanup_old_captures(tmp.path());
        assert_eq!(removed, 0);
    }

    #[test]
    fn cleanup_old_captures_leaves_fresh_files_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let fresh = tmp.path().join("fresh.png");
        std::fs::write(&fresh, b"x").unwrap();
        let removed = cleanup_old_captures(tmp.path());
        assert_eq!(removed, 0, "fresh files (mtime ~now) must be retained");
        assert!(fresh.exists());
    }

    #[test]
    fn cleanup_old_captures_silently_handles_missing_dir() {
        let removed = cleanup_old_captures(Path::new("/nonexistent/captures/dir"));
        assert_eq!(removed, 0, "missing dir is not an error — return 0");
    }

    #[test]
    fn capture_error_serializes_with_named_kind_per_falsifier_a_and_j() {
        let err = CaptureError::ElectronNotRunning;
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("\"kind\":\"electron_not_running\""), "got: {s}");
        let err = CaptureError::WindowMinimized;
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("\"kind\":\"window_minimized\""), "got: {s}");
        let err = CaptureError::NoWindow;
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("\"kind\":\"no_window\""), "got: {s}");
    }

    #[test]
    fn parse_window_discovery_output_extracts_wid_width_height_for_visible_window() {
        let parsed = parse_window_discovery_output("12345 1280 800").unwrap();
        assert_eq!(
            parsed,
            DiscoveredWindow {
                window_id: 12345,
                width: 1280,
                height: 800,
            }
        );
    }

    #[test]
    fn parse_window_discovery_output_no_window_marker_returns_no_window_error() {
        let err = parse_window_discovery_output("NO_WINDOW").unwrap_err();
        assert_eq!(err, CaptureError::NoWindow);
    }

    #[test]
    fn parse_window_discovery_output_no_electron_marker_returns_electron_not_running() {
        let err = parse_window_discovery_output("NO_ELECTRON").unwrap_err();
        assert_eq!(err, CaptureError::ElectronNotRunning);
    }

    #[test]
    fn parse_window_discovery_output_zero_dimensions_means_window_minimized() {
        let err = parse_window_discovery_output("12345 0 0").unwrap_err();
        assert_eq!(err, CaptureError::WindowMinimized);
    }

    #[test]
    fn parse_window_discovery_output_takes_last_line_so_swift_warnings_do_not_break_it() {
        let parsed =
            parse_window_discovery_output("warning: deprecated API\n12345 1280 800").unwrap();
        assert_eq!(parsed.window_id, 12345);
    }

    #[test]
    fn classify_screencapture_failure_recognizes_screen_recording_perm_denial() {
        let err = classify_screencapture_failure("could not create image from window", Some(1));
        assert!(matches!(err, CaptureError::PermissionDenied { .. }));
        let err = classify_screencapture_failure("Not authorized to capture", Some(1));
        assert!(matches!(err, CaptureError::PermissionDenied { .. }));
    }

    #[test]
    fn classify_screencapture_failure_falls_back_to_capture_failed_for_other_errors() {
        let err = classify_screencapture_failure("disk full", Some(1));
        assert!(
            matches!(err, CaptureError::CaptureFailed { .. }),
            "non-permission errors must map to capture_failed, not permission_denied"
        );
    }

    #[test]
    fn permission_denied_serializes_with_grant_instructions_in_detail() {
        let err = CaptureError::PermissionDenied {
            detail: "Grant Screen Recording permission".to_string(),
        };
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("\"kind\":\"permission_denied\""), "got: {s}");
        assert!(
            s.contains("Screen Recording"),
            "agent must see grant-path in detail; got: {s}"
        );
    }
}

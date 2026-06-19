use super::CaptureError;

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

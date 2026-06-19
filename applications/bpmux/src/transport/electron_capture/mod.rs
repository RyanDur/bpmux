mod screenshot;
mod window;

#[cfg(test)]
mod tests;

pub use screenshot::{
    build_capture_filename, capture_window_to_path, captures_dir, classify_screencapture_failure,
    cleanup_old_captures,
};
pub use window::{DiscoveredWindow, find_bpmux_window, parse_window_discovery_output};

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

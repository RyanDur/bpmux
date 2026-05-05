//! Live-Electron integration test for capture_electron_window MCP
//! tool (#1183). Run locally on macOS with:
//!
//!     cargo test -p bpmux --test integration_main capture_electron_window
//!
//! Requires: bpmux Electron app running, macOS Screen Recording
//! permission granted to the terminal/process running this test.
//!
//! Per #1183 falsifier-g (`feedback_test_fidelity_vocabulary_discipline` +
//! `feedback_visual_class_falsifier_needs_visual_assertion`): mock-only
//! is the substitution-class. Live test below verifies (1) window
//! discovery returns a real CGWindowID for the bpmux Electron, (2)
//! `screencapture` writes an actual PNG file at the namespaced path,
//! (3) the PNG has non-zero size + the PNG file signature.
//!
//! Cross-platform CI gate: gated on `target_os = "macos"` so Linux
//! CI doesn't try to run it (`swift` + Quartz + Electron all absent).
//! Inside the macOS gate, soft-skip when no bpmux Electron window is
//! visible — covers macOS CI runners (no GUI Electron) + macOS dev
//! without Electron currently running. The test asserts only when
//! the prerequisite environment exists, so CI green is achievable
//! anywhere; only running-Electron + Screen Recording grant unlocks
//! the full assertion path. #1193 removed the prior
//! `integration_tests` Cargo feature gate (Ryan 2026-05-05 directive
//! against tests calling outside-this-computer resources). Quartz +
//! Electron are local-this-computer, so the test stays — it's the
//! soft-skip + macOS-only target gate that keeps CI safe.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "macos")]
mod live {
    use bpmux::transport::electron_capture::{
        CaptureError, build_capture_filename, capture_window_to_path, captures_dir,
        find_bpmux_window,
    };

    fn read_png_signature(path: &std::path::Path) -> [u8; 8] {
        let bytes = std::fs::read(path).expect("read captured png");
        assert!(bytes.len() >= 8, "captured png is too small to be valid");
        let mut sig = [0u8; 8];
        sig.copy_from_slice(&bytes[..8]);
        sig
    }

    #[tokio::test]
    async fn live_capture_against_running_electron_writes_valid_png_at_namespaced_path() {
        let window = match find_bpmux_window().await {
            Ok(w) => w,
            Err(CaptureError::ElectronNotRunning) | Err(CaptureError::NoWindow) => {
                eprintln!(
                    "skip: no bpmux Electron window on screen — full live-walk requires \
                     running Electron + Screen Recording grant; substrate verified via \
                     unit tests"
                );
                return;
            }
            Err(CaptureError::SwiftUnavailable { detail }) => {
                eprintln!("skip: swift unavailable on this host — {detail}");
                return;
            }
            Err(other) => {
                panic!("unexpected discovery error: {other:?}");
            }
        };
        assert!(window.window_id > 0, "window_id must be a real CGWindowID");
        assert!(
            window.width >= 200 && window.height >= 200,
            "discovered window dimensions must be plausibly bpmux-sized; got {}x{}",
            window.width,
            window.height
        );

        let dir = captures_dir().expect("captures dir resolvable");
        let now = chrono::Utc::now();
        let filename = build_capture_filename("integration-test", now);
        let path = dir.join(&filename);

        match capture_window_to_path(window.window_id, &path).await {
            Ok(()) => {}
            Err(CaptureError::PermissionDenied { detail }) => {
                eprintln!(
                    "skip: screencapture permission denied — {detail}. \
                     Substrate verified up to the boundary; full live-walk requires \
                     Screen Recording grant in System Settings."
                );
                return;
            }
            Err(other) => panic!("unexpected capture error: {other:?}"),
        }

        assert!(
            path.exists(),
            "capture must write the PNG at the namespaced path"
        );
        let sig = read_png_signature(&path);
        assert_eq!(
            &sig,
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "captured file must have PNG signature (\\x89PNG\\r\\n\\x1a\\n) — \
             confirms screencapture wrote real image data, not a stub"
        );

        let _ = std::fs::remove_file(&path);
    }
}

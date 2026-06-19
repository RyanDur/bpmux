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

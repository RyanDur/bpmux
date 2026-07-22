#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

pub(super) struct CwdGuard {
    saved: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl CwdGuard {
    pub(super) fn enter(target: &std::path::Path) -> Self {
        let guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::current_dir().unwrap();
        std::env::set_current_dir(target).unwrap();
        Self {
            saved,
            _guard: guard,
        }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.saved);
    }
}

fn workspace_functions_dir_for_test(cwd: &std::path::Path) -> PathBuf {
    cwd.join(".bpmux").join("functions")
}

#[test]
fn workspace_functions_dir_for_test_helper_returns_relative_path() {
    let p = std::path::Path::new("/foo/bpmux-dev");
    assert_eq!(
        workspace_functions_dir_for_test(p),
        std::path::Path::new("/foo/bpmux-dev/.bpmux/functions")
    );
}

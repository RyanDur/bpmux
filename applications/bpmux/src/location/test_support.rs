use std::path::Path;
use std::sync::{Mutex, MutexGuard};

static DATA_BASE_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct DataBaseGuard {
    prev: Option<String>,
    _lock: MutexGuard<'static, ()>,
}

impl DataBaseGuard {
    pub(crate) fn set(path: &Path) -> Self {
        let lock = DATA_BASE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let prev = std::env::var("BPMUX_DATA_BASE").ok();
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("BPMUX_DATA_BASE", path);
        }
        Self { prev, _lock: lock }
    }
}

impl Drop for DataBaseGuard {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            match &self.prev {
                Some(value) => std::env::set_var("BPMUX_DATA_BASE", value),
                None => std::env::remove_var("BPMUX_DATA_BASE"),
            }
        }
    }
}

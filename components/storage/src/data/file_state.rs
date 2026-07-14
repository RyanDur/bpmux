use super::FileStorage;
use crate::Storage;

impl Storage for FileStorage {
    fn load_state(&self, key: &str) -> Option<String> {
        let safe_key = memory::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        std::fs::read_to_string(&path).ok()
    }

    fn save_state(&self, key: &str, value: &str) {
        let safe_key = memory::identity::sanitize_name(key);
        let path = self.dir.join(format!("{safe_key}.json"));
        let tmp = self.dir.join(format!("{safe_key}.json.tmp"));
        if let Err(e) = std::fs::write(&tmp, value) {
            diag::log_diag(&format!("save_state write failed for {safe_key}: {e}"));
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            diag::log_diag(&format!("save_state rename failed for {safe_key}: {e}"));
        }
    }
}

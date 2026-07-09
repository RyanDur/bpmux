use std::collections::HashSet;

use super::FileStorage;

impl crate::awareness::SurfacedTopicsRepo for FileStorage {
    fn load_surfaced_topics(&self) -> HashSet<String> {
        let path = self.dir.join("surfaced-topics.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join("surfaced-topics.json");
        let json = serde_json::to_string(topics).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

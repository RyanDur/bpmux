use super::App;

impl App {
    /// Return the filtered list of palette actions matching
    /// the current palette_input. Fuzzy match: the input chars must
    /// appear in order in the action name, but not necessarily
    /// adjacent.
    pub fn palette_matches(&self) -> Vec<(&'static str, &'static str)> {
        use crate::terminal::input::SLASH_COMMANDS;
        let query = self.palette.input.to_lowercase();
        if query.is_empty() {
            return SLASH_COMMANDS
                .iter()
                .map(|c| (c.name, c.short_desc))
                .collect();
        }
        SLASH_COMMANDS
            .iter()
            .filter_map(|c| {
                let haystack = c.name.to_lowercase();
                let mut hay_iter = haystack.chars();
                let matches = query.chars().all(|qc| hay_iter.any(|hc| hc == qc));
                if matches {
                    Some((c.name, c.short_desc))
                } else {
                    None
                }
            })
            .collect()
    }
}

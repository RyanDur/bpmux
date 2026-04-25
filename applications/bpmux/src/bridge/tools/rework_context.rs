pub fn enrich_body(original_body: &str, context: &str) -> String {
    format!("{context}\n{original_body}")
}

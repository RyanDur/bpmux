use std::future::Future;
use std::pin::Pin;

use chrono::Utc;

use product_management::story::events::EventMeta;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub(crate) fn cache_fill_meta() -> EventMeta {
    EventMeta {
        actor: "source-ingest".to_string(),
        actor_id: None,
        ts: Utc::now(),
    }
}

#[cfg(test)]
mod cache_fill_meta_tests {
    use super::cache_fill_meta;

    #[test]
    fn cache_fill_meta_stamps_no_actor_id() {
        let meta = cache_fill_meta();
        assert_eq!(meta.actor, "source-ingest");
        assert_eq!(
            meta.actor_id, None,
            "source-ingest has no user-identity lane; actor_id stays explicitly None"
        );
    }
}

use crate::product_management::history::{SharedHistoryAction, VerdictRecord};
use crate::store::AppAction;
use crate::sync::RefreshDriver;

pub(crate) async fn backfill_shared_history(
    refresh: &dyn RefreshDriver,
    dispatch: impl Fn(AppAction),
) {
    for story in refresh.all_stories() {
        let number = story.id.value();
        match refresh.fetch_verdicts(number).await {
            Ok(verdicts) => {
                let records: Vec<VerdictRecord> = verdicts
                    .iter()
                    .map(|verdict| VerdictRecord::from_cast(story.id, verdict))
                    .collect();
                dispatch(AppAction::SharedHistory(SharedHistoryAction::Backfilled {
                    story: story.id,
                    records,
                }));
            }
            Err(e) => {
                crate::diag::log_diag(&format!(
                    "backfill: fetch_verdicts(#{number}) failed: {e}; shared history stays thin for this story"
                ));
            }
        }
    }
}

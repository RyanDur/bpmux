use crate::store::AppAction;
use crate::sync::RefreshDriver;
use product_management::history::{SharedHistoryAction, VerdictRecord};

pub(crate) async fn backfill_shared_history(
    refresh: &dyn RefreshDriver,
    select: &crate::store::Select,
    dispatch: impl Fn(AppAction),
) {
    let stories = select.select(|state| product_management::story::slice::all(&state.stories));
    for story in stories {
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
                diag::log_diag(&format!(
                    "backfill: fetch_verdicts(#{number}) failed: {e}; shared history stays thin for this story"
                ));
            }
        }
    }
}

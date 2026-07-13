use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::owner_session::OwnerSessionAction;

#[derive(Deserialize)]
pub struct ClaimOwnerSessionRequest {
    pub nonce: String,
}

pub async fn post_owner_session(
    Extension(dispatch): Extension<crate::store::Dispatch>,
    Json(payload): Json<ClaimOwnerSessionRequest>,
) -> impl IntoResponse {
    dispatch.dispatch(
        OwnerSessionAction::Claimed {
            nonce: payload.nonce,
        }
        .into(),
    );
    (StatusCode::CREATED, "owner session claimed".to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::store::AppAction;

    #[tokio::test]
    async fn a_claim_dispatches_the_nonce_into_the_owner_session_slice() {
        let (dispatch, mut rx) = crate::store::Dispatch::recording();
        let response = post_owner_session(
            Extension(dispatch),
            Json(ClaimOwnerSessionRequest {
                nonce: "launch-nonce-1".to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        match rx.try_recv().expect("the claim emits exactly one action") {
            AppAction::OwnerSession(OwnerSessionAction::Claimed { nonce }) => {
                assert_eq!(
                    nonce, "launch-nonce-1",
                    "the endpoint installs the launcher's nonce as the current owner nonce"
                );
            }
            _ => panic!("expected an OwnerSession Claimed action, got a different variant"),
        }
    }
}

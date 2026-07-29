use super::errors::{BoardFetchError, classify_response};

#[test]
fn a_refused_token_reads_human_and_never_promises_a_restart() {
    let message = BoardFetchError::AuthInvalid.user_message();
    assert_eq!(
        message,
        "GitHub refused the token — check it in GitHub settings and reconnect"
    );
    assert!(!message.contains("restart"));
}

#[test]
fn raw_responses_classify_like_client_errors() {
    assert_eq!(
        classify_response(401, r#"{"message":"Bad credentials"}"#),
        BoardFetchError::AuthInvalid
    );
    assert_eq!(
        classify_response(403, r#"{"message":"API rate limit exceeded"}"#),
        BoardFetchError::RateLimited {
            retry_after_secs: None
        }
    );
    assert_eq!(
        classify_response(429, "slow down"),
        BoardFetchError::RateLimited {
            retry_after_secs: None
        }
    );
    assert_eq!(
        classify_response(500, "boom"),
        BoardFetchError::Other("github error 500: boom".to_string())
    );
}

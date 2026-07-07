use super::SessionRouter;
use crate::transport;

pub(in crate::server) fn test_seed_routes() -> SessionRouter {
    if !transport::test_seed::seed_api_enabled() {
        return axum::Router::new();
    }
    axum::Router::new()
        .route(
            "/api/test/seed_messages",
            axum::routing::post(transport::test_seed::post_seed_messages),
        )
        .route(
            "/api/test/seed_reactions",
            axum::routing::post(transport::test_seed::post_seed_reactions),
        )
        .route(
            "/api/test/reset_messages",
            axum::routing::post(transport::test_seed::post_reset_messages),
        )
        .route(
            "/api/test/seed_roster",
            axum::routing::post(transport::test_seed::post_seed_roster),
        )
        .route(
            "/api/test/seed_stories",
            axum::routing::post(transport::test_seed::post_seed_stories),
        )
        .route(
            "/api/test/reset_stories",
            axum::routing::post(transport::test_seed::post_reset_stories),
        )
        .route(
            "/api/test/fire_ci_state",
            axum::routing::post(transport::test_routes::post_test_fire_ci_state),
        )
        .route(
            "/api/test/fire_sync_status",
            axum::routing::post(transport::test_routes::post_test_fire_sync_status),
        )
}

pub(in crate::server) fn test_routes(enabled: bool) -> SessionRouter {
    if !enabled {
        return axum::Router::new();
    }
    axum::Router::new()
        .route(
            "/api/test/create_story",
            axum::routing::post(transport::test_routes::post_test_create_story),
        )
        .route(
            "/api/test/create_iteration",
            axum::routing::post(transport::test_routes::post_test_create_iteration),
        )
        .route(
            "/api/test/designate_current_iteration",
            axum::routing::post(transport::test_routes::post_test_designate_current_iteration),
        )
        .route(
            "/api/test/move_story",
            axum::routing::post(transport::test_routes::post_test_move_story),
        )
        .route(
            "/api/test/edit_story",
            axum::routing::post(transport::test_routes::post_test_edit_story),
        )
        .route(
            "/api/test/comment_on_story",
            axum::routing::post(transport::test_routes::post_test_comment_on_story),
        )
        .route(
            "/api/test/close_story",
            axum::routing::post(transport::test_routes::post_test_close_story),
        )
        .route(
            "/api/test/close_iteration",
            axum::routing::post(transport::test_routes::post_test_close_iteration),
        )
        .route(
            "/api/test/deliver_story",
            axum::routing::post(transport::test_routes::post_test_deliver_story),
        )
        .route(
            "/api/test/start_story",
            axum::routing::post(transport::test_routes::post_test_start_story),
        )
        .route(
            "/api/test/accept_story",
            axum::routing::post(transport::test_routes::post_test_accept_story),
        )
        .route(
            "/api/test/fail_desk_check",
            axum::routing::post(transport::test_routes::post_test_fail_desk_check),
        )
}

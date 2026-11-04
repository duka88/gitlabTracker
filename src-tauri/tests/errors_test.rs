//! Error mapping (contracts/gitlab-graphql.md) and the status-filter retry decision.
use gitlab_tracker_lib::error::AppError;
use gitlab_tracker_lib::gitlab::client::{classify, is_status_filter_error};
use serde_json::json;

#[test]
fn http_statuses() {
    assert_eq!(classify(401, json!(null)), Err(AppError::TokenRejected));
    assert_eq!(classify(403, json!({})), Err(AppError::NoPermission));
    assert_eq!(classify(429, json!({})), Err(AppError::RateLimited));
    assert!(matches!(classify(502, json!(null)), Err(AppError::Query(m)) if m.contains("502")));
}

#[test]
fn graphql_errors_and_data() {
    assert_eq!(
        classify(
            200,
            json!({ "errors": [{ "message": "Field 'x' doesn't exist" }], "data": null })
        ),
        Err(AppError::Query("Field 'x' doesn't exist".into()))
    );
    assert_eq!(
        classify(200, json!({ "errors": [{ "message": "Invalid token" }] })),
        Err(AppError::TokenRejected)
    );
    assert_eq!(
        classify(200, json!({ "data": { "a": 1 } })),
        Ok(json!({ "a": 1 }))
    );
    assert!(classify(200, json!({ "data": null })).is_err());
}

#[test]
fn messages_and_kinds() {
    assert_eq!(
        AppError::TokenRejected.to_string(),
        "Token rejected — update it in Settings"
    );
    assert_eq!(
        AppError::Unreachable.to_string(),
        "Can't reach GitLab — showing last data"
    );
    assert_eq!(
        AppError::RateLimited.to_string(),
        "GitLab rate limit — retrying next check"
    );
    assert_eq!(
        AppError::NoPermission.to_string(),
        "No permission to read this list"
    );
    assert!(AppError::TokenRejected.is_global() && AppError::Unreachable.is_global());
    assert!(!AppError::NotFound("Group not found".into()).is_global());
    assert!(AppError::RateLimited.is_warning());
    let wire = serde_json::to_value(AppError::NotFound("Project not found".into())).unwrap();
    assert_eq!(
        wire,
        json!({ "kind": "notFound", "message": "Project not found" })
    );
}

#[test]
fn status_filter_errors() {
    let yes = [
        "Field 'workItems' doesn't accept argument 'status'",
        "Variable $status of type WorkItemWidgetStatusFilterInput was provided invalid value",
        "Status filter is not available for this namespace",
    ];
    for m in yes {
        assert!(is_status_filter_error(&AppError::Query(m.into())), "{m}");
    }
    let no = [
        "Field 'labels' doesn't accept argument 'x'",
        "Something went wrong",
    ];
    for m in no {
        assert!(!is_status_filter_error(&AppError::Query(m.into())), "{m}");
    }
    assert!(!is_status_filter_error(&AppError::Unreachable));
}

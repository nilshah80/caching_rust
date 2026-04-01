//! Request-scoped context helpers.
//!
//! Stores per-request metadata in a task-local so response builders and
//! error conversion can access the active request ID without threading it
//! through every handler and service signature.

use std::future::Future;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Run a future with the given request ID bound to the current task.
pub async fn scope_request_id<F>(request_id: String, future: F) -> F::Output
where
    F: Future,
{
    REQUEST_ID.scope(request_id, future).await
}

/// Get the current request ID from the active task context.
pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(Clone::clone).ok()
}

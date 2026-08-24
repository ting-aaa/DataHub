use std::future::Future;

use uuid::Uuid;

tokio::task_local! {
    static CORRELATION_ID: Uuid;
}

/// Runs a request or job future with a task-local correlation identifier.
pub async fn scope_correlation<T>(id: Uuid, future: impl Future<Output = T>) -> T {
    CORRELATION_ID.scope(id, future).await
}

/// Returns the correlation identifier for the current asynchronous task.
#[must_use]
pub fn current_correlation_id() -> Option<Uuid> {
    CORRELATION_ID.try_with(|id| *id).ok()
}

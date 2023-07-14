// Public Headers. Those will be seen by our users.
pub const REQUEST_ID_HEADER: &str = "x-cronback-request-id";
pub const PROJECT_ID_HEADER: &str = "x-cronback-project-id";

// Those are internal cross-service headers.
pub const PARENT_SPAN_HEADER: &str = "x-cronback-parent-span-id";
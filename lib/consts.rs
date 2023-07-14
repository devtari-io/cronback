// Public Headers. Those will be seen by our users.
pub const REQUEST_ID_HEADER: &str = "x-cronback-request-id";
pub const PROJECT_ID_HEADER: &str = "x-cronback-project-id";

// Those are internal cross-service headers.
pub const PARENT_SPAN_HEADER: &str = "x-cronback-parent-span-id";

// Headers we send out to webhook endpoints
pub const DELIVERY_ATTEMPT_NUM_HEADER: &str =
    "x-cronback-delivery-attempt-number";
pub const RUN_ID_HEADER: &str = "x-cronback-run-id";

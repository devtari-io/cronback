// Public Headers. Those will be seen by our users.
pub static REQUEST_ID_HEADER: &str = "x-cronback-request-id";
pub static PROJECT_ID_HEADER: &str = "x-cronback-project-id";

// Those are internal cross-service headers.
pub static PARENT_SPAN_HEADER: &str = "x-cronback-parent-span-id";

// Headers we send out to webhook endpoints
pub static DELIVERY_ATTEMPT_NUM_HEADER: &str =
    "x-cronback-delivery-attempt-number";
pub static RUN_ID_HEADER: &str = "x-cronback-run-id";

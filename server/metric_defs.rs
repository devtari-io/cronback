use metrics::{describe_counter, describe_histogram};

pub(crate) fn install_metrics() {
    describe_counter!(
        "cronback.scheduler.rpc_request_total",
        "Total RPC requests processed"
    );
    describe_histogram!(
        "cronback.scheduler.rpc_total_latency_ms",
        "Total latency of RPC processing in millis"
    );
}

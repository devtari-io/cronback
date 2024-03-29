use metrics::{describe_counter, describe_histogram, Unit};

/// Optional but adds description/help message to the metrics emitted to metric
/// sink.
pub(crate) fn install_metrics() {
    // Core metrics
    describe_counter!(
        "rpc.requests_total",
        Unit::Count,
        "Total RPC requests processed"
    );
    describe_histogram!(
        "rpc.duration_seconds",
        Unit::Seconds,
        "Total latency of RPC processing in seconds"
    );
}

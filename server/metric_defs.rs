use metrics::{describe_counter, describe_gauge, describe_histogram, Unit};

/// Optional but adds description/help message to the metrics emitted to metric sink.
pub(crate) fn install_metrics() {
    describe_counter!(
        "cronback.rpc.requests_total",
        Unit::Count,
        "Total RPC requests processed"
    );
    describe_histogram!(
        "cronback.rpc.duration_seconds",
        Unit::Seconds,
        "Total latency of RPC processing in seconds"
    );
    describe_histogram!(
        "cronback.spinner.yield_duration_ms",
        Unit::Milliseconds,
        "The time where the spinner gets to sleep until next tick"
    );
    describe_histogram!(
        "cronback.spinner.dispatch_lag_seconds",
        Unit::Seconds,
        "How many seconds the spinner is lagging from trigger ticks"
    );
    describe_gauge!(
        "cronback.spinner.active_triggers_total",
        Unit::Count,
        "How many active triggers are loaded into the spinner"
    );
}

use metrics::{describe_counter, describe_gauge, describe_histogram, Unit};

/// Optional but adds description/help message to the metrics emitted to metric
/// sink.
pub(crate) fn install_metrics() {
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
    describe_histogram!(
        "spinner.yield_duration_ms",
        Unit::Milliseconds,
        "The time where the spinner gets to sleep until next tick"
    );
    describe_histogram!(
        "spinner.dispatch_lag_seconds",
        Unit::Seconds,
        "How many seconds the spinner is lagging from trigger ticks"
    );
    describe_gauge!(
        "spinner.active_triggers_total",
        Unit::Count,
        "How many active triggers are loaded into the spinner"
    );

    // API Server
    describe_counter!(
        "api.http_requests_total",
        Unit::Count,
        "Total HTTP API requests processed"
    );
    describe_histogram!(
        "api.http_requests_duration_seconds",
        Unit::Seconds,
        "Total HTTP API processing in seconds"
    );

    // Dipatcher
    describe_counter!(
        "dispatcher.runs_total",
        Unit::Count,
        "Total number of runs by the dispatcher"
    );
    describe_counter!(
        "dispatcher.attempts_total",
        Unit::Count,
        "Total number of attempts attempted by the dispatcher"
    );

    describe_gauge!(
        "dispatcher.inflight_runs_total",
        Unit::Count,
        "Total number of inflight runs in the dispatcher"
    );
}

# cronback
Allows developers to schedule webhooks on-demand, with backoff, or on a cron recurring schedule.


# Documentation
Documentation (/doc) is created with `mdbook`. Make sure that you have `cargo install mdbook`.


# Build
Install protobuf
```
brew install protobuf
```

# Tests
```
cargo nextest run --workspace
```

# Run
```
cargo run -- -c <config-file>

# Skip missed invocations by 

CRONBACK__SCHEDULER__DANGEROUS_FAST_FORWARD=true cargo run

# Set the admin key
CRONBACK__API__ADMIN_API_KEYS=adminkey cargo run

```

# Prometheus

```
curl http://localhost:9000/metrics
```

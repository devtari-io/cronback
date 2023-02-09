# cronback
Allows developers to schedule webhooks on-demand, with backoff, or on a cron recurring schedule.


# Documentation
Documentation (/doc) is created with `mdbook`. Make sure that you have `cargo install mdbook`.


# Build
Install protobuf
```
brew install protobuf
```

# Run
```
cargo run -- -c <config-file>
```

# Prometheus

```
curl http://localhost:9000/metrics
```

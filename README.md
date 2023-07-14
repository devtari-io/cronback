# cronback

Allows developers to schedule webhooks on-demand, with backoff, or on a cron recurring schedule.

# Build

Install protobuf

```
brew install protobuf
```

# Tests

Install the cargo-nextest test runner [link](https://nexte.st/book/installation.html)
```
cargo nextest run --workspace
```

# Run

```
cargo run -- -c <config-file>

# Skip missed runs by

CRONBACK__SCHEDULER__DANGEROUS_FAST_FORWARD=true cargo run

# Set the admin key
CRONBACK__API__ADMIN_API_KEYS=adminkey cargo run

# Get API Secret Token:
export CRONBACK_SECRET_TOKEN=`http -j --auth adminkey --auth-type bearer http://localhost:8888/v1/admin/api_keys X-On-Behalf-Of:prj_063001GZKTEF61EJ34W1G0PXJS7V6M key_name=master | jq -r '.key'`

```

# Prometheus

```
curl http://localhost:9000/metrics
```

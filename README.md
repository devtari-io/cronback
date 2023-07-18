# cronback


> **Allows developers to schedule webhooks on-demand, with backoff, or on a cron recurring schedule.**


[![License](https://img.shields.io/badge/license-BSD--2--Clause--Patent-blue?style=flat-square
)](LICENSE)


## Build

Install protobuf

```sh
brew install protobuf
cargo build
```

## Tests

Install the cargo-nextest test runner [link](https://nexte.st/book/installation.html)
```sh
cargo nextest run
```

## Run in Development Environment

1/ Provision the sqlite database:
```sh
cargo run -p cronback-migration -- up -u sqlite://database.sqlite
```


2/ Run the server with the default configuration
```sh
# Set the admin key
CRONBACK__API__ADMIN_API_KEYS=adminkey cargo run
```

```sh
# Skip missed runs by

CRONBACK__SCHEDULER__DANGEROUS_FAST_FORWARD=true cargo run

```

3/ Get API Secret Token:
```sh
cargo cli --localhost --secret-token=adminkey admin --project-id=prj_063001GZKTEF61EJ34W1G0PXJS7V6M api-keys create dev_key
```

## Prometheus Metrics

```
curl http://localhost:9000/metrics
```

# License
The software is distributed under the terms of the GPLv2-compatible, OSI-approved, 
permissive [BSD-2-Clause-Patent](LICENSE) license.

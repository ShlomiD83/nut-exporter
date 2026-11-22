# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Optional background watcher (`WATCH_TARGET`): checks NUT every 2 s and exposes `nut_on_battery_events_total`, `nut_on_battery_seconds_total` `nut_last_on_battery_timestamp_seconds` and `nut_watcher_up` (a Watcher card on the dashboards), so outages shorter than the Prometheus scrape interval are counted.
- Grafana dashboards in ₪, $ and € (`dashboards/nut-exporter-charging-status-{ils,usd,eur}.json`) with power, outage timeline/count/time on battery, energy and cost for this or last month (Cost period switch, price and discount settings), and On battery markers.
- CI integration test simulates a short outage and checks the watcher counts it.
- HTTP/2 support (hyper-util auto connection builder).
- `cargo audit` security check and Dependabot in CI.
- Dependabot PRs are merged automatically once CI passes, and the new image is published.

### Changed

- When the NUT server can't be reached the exporter answers HTTP 200 with `nut_scrape_success 0` (and keeps the watcher metrics) instead of HTTP 503. The NUT query is bounded by Prometheus' scrape timeout (`X-Prometheus-Scrape-Timeout-Seconds`). Dashboards show *NUT unreachable* on the Status card.
- Migrated to hyper 1.x / hyper-util / http-body-util and updated all dependencies.
- Docker image is now `FROM scratch` with a static musl binary, published as `romekd/prometheus-nut-exporter` (`latest` + date tag, amd64 + arm64).
- Smaller binary: full symbol stripping, LTO and a single codegen unit (amd64 image ~7.4 MB -> ~4.3 MB).
- Builder stage uses the official `rust:<version>-alpine` image, so Dependabot keeps Rust and Alpine up to date.

### Deprecated

### Removed

- Unused `chrono` dependency; `url` replaced by `form_urlencoded`.

### Fixed

- NUT queries now time out (5 s to connect, 10 s for the exchange) instead of hanging when the NUT server or network stops responding.

### Security

## [1.2.1] - 2022-08-03

### Added

- Added `BOOST` status to the `nut_ups_status` state set.

## [1.2.0] - 2022-07-29

### Added

- Added metrics (@uncleBorsch):
    - `nut_temperature_celsius` (`ups.temperature`)
    - `nut_delay_shutdown_seconds` (`ups.delay.shutdown`)
    - `nut_delay_start_seconds` (`ups.delay.start`)
    - `nut_battery_voltage_high_volts` (`battery.voltage.high`)
    - `nut_battery_voltage_low_volts` (`battery.voltage.low`)
- Added more UPS metadata (@uncleBorsch).
- Added support for binding to a specific IP address through the `HTTP_ADDRESS` environment variable (@nsapa).
- Added metric `nut_ups_status` as a state set with support for many UPS statuses (`OL`, `OB`, `LB`, `CHRG` etc.).
- Added labels `driver_version`, `driver_version_internal`, `driver_version_data` and `manufacturing_date` to the `nut_ups_info` metric.
- Added proper signal handling to shutdown gracefully and not hang.
- Added multi-architecture support (Docker images for different architectures will get published to Docker Hub).

### Changed

- Changed default log level to `info`.
- Made the target port default to 3493 instead of requiring one to be provided.
- Made the Prometheus/OpenMetrics output (more) OpenMetrics 1.0.0-compliant.

### Deprecated

- Deprecated `nut_info` and added `nut_server_info` as an identical but better named replacement.
- Deprecated `nut_status` as it has very limited support for UPS statuses and doesn't always work as intended. See `nut_ups_status` for the replacement.
- Deprecated the `type` and `nut_version` labels from the `nut_ups_info` metric. Use `device_type` and `driver_version` instead.
- Deprecated `nut_battery_volts`, `nut_input_volts` and `nut_output_volts`. Replacements were added in v1.1.0.

### Removed

- Removed the tini init system from the Docker image, since signals are handled properly now.

### Fixed

- Fixed typo in the `device.mfr` variable (@nsapa).

## [1.1.1] - 2021-04-11

### Added

- Added proper logging with adjustable log level.
- Added duplicate compatibility metrics to compensate for the renamed metrics in the previous release.

### Changed

- Changed request logging to use log level `debug` instead of the `LOG_REQUESTS_CONSOLE` environment variable.

### Fixed

- Fixed failing to parse non-semantic NUT versions.

## [1.1.0] - 2021-04-07

### Added

- Added UPS description to `nut_ups_info`.
- Added lots of more metrics.

### Changed

- Replaced Docker image tags `stable` and `bleeding` with `latest`, `X`, `X.Y` and `X.Y.Z` (parts of the semantic version).
- Renamed a few voltage-related metrics (slightly breaking).

### Fixed

- Added Tini as container entrypoint to handle signals properly (i.e. not stall when exiting).
- Fixed parsing error when multiple UPSes exist.

## [1.0.1] - 2020-06-29

### Added

- Added metadata metrics `nut_info` for the NUT server and `nut_exporter_info` for the exporter.

### Changed

- Improved error messages sent to client.

### Fixed

- Fixed malformed labels for `nut_ups_info`.

## [1.0.0] - 2020-06-18

Initial release.

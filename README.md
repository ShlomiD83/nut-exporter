<p align="center"><img src="https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/docs/logo.png" width="128" height="128" alt="Prometheus NUT Exporter logo"></p>

# Prometheus NUT Exporter (modernized amd64 + arm64 build)

[![CI](https://github.com/ShlomiD83/nut-exporter/actions/workflows/ci.yml/badge.svg)](https://github.com/ShlomiD83/nut-exporter/actions/workflows/ci.yml)
[![Docker pulls](https://img.shields.io/docker/pulls/romekd/prometheus-nut-exporter?label=Docker%20Hub)](https://hub.docker.com/r/romekd/prometheus-nut-exporter)

**📊 Grafana dashboards** (identical except for the cost currency): [₪ ILS](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-ils.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-ils.json)) · [$ USD](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-usd.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-usd.json)) · [€ EUR](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-eur.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-eur.json)). Import one in Grafana via Dashboards → New → Import, then pick your Prometheus data source.

> **⚠️ IMPORTANT – Grafana plugin required**
>
> The dashboards show UPS runtime as `mm:ss` using the **Dynamic Text** panel plugin. Grafana must have it installed, or the runtime panel will not render. Set this environment variable on the **Grafana** container:
>
> ```
> GF_PLUGINS_PREINSTALL=marcusolsson-dynamictext-panel
> ```
>
> See [Grafana](#grafana) below for a full example.

Fork of [HON95/prometheus-nut-exporter](https://github.com/HON95/prometheus-nut-exporter), published as [`romekd/prometheus-nut-exporter`](https://hub.docker.com/r/romekd/prometheus-nut-exporter) on Docker Hub and [`ghcr.io/shlomid83/nut-exporter`](https://github.com/ShlomiD83/nut-exporter/pkgs/container/nut-exporter) on GitHub Container Registry (same image, same tags).

Differences from upstream:

- Updated to current dependencies (hyper 1.x via hyper-util, tokio 1.5x, env_logger 0.11); unused `chrono`/`url` dependencies removed.
- HTTP server serves both HTTP/1.1 and HTTP/2.
- Optional background watcher (`WATCH_TARGET`) that checks NUT every 2 s and counts switches to battery, so even a few-second power blip shows up.
- NUT queries time out (5 s connect, 10 s exchange) instead of hanging.
- Fully static musl binary in a `FROM scratch` image (no OS layer), running as UID/GID 5000.
- Image tags: `latest` and a build date tag (`YYYY-MM-DD`), multi-arch for `linux/amd64` and `linux/arm64`. A new image is published only when the code, Cargo files or Dockerfile change (or on a manual CI run); README-only changes just update the Docker Hub description.
- CI runs clippy, an integration test against a mock NUT server, and `cargo audit` (RustSec advisories) on every push and weekly.
- Dependabot opens weekly dependency PRs; they are merged automatically once CI passes, which also publishes a new image.

Configuration, metrics and Prometheus setup are compatible with upstream (the watcher only adds metrics) and documented below.

![Charging Status dashboard](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/docs/screenshots/dashboard-ils.png)

Screenshots (demo data): [₪ ILS](https://github.com/ShlomiD83/nut-exporter/blob/main/docs/screenshots/dashboard-ils.png) · [$ USD](https://github.com/ShlomiD83/nut-exporter/blob/main/docs/screenshots/dashboard-usd.png) · [€ EUR](https://github.com/ShlomiD83/nut-exporter/blob/main/docs/screenshots/dashboard-eur.png)

A Prometheus/OpenMetrics exporter for uninterruptable power supplies (UPSes) using Network UPS Tools (NUT).

## Usage

### NUT

Set up NUT in server mode and make sure the TCP port (3493 by default) is accessible (without authentication).

If you want to test that it's working, run `telnet <nut-server> 3493` and then `VER`, `LIST UPS` and `LIST VAR <ups>`.

### Docker

Example `docker-compose.yml`:

```yaml
services:
  nut-exporter:
    image: romekd/prometheus-nut-exporter:latest
    container_name: nut-exporter
    restart: unless-stopped
    environment:
      # /metrics is Prometheus' default metrics_path, so no extra scrape config is needed
      HTTP_PATH: /metrics
      RUST_LOG: info
      # Check the NUT server every 2 s so short outages are counted (same address as the Prometheus target)
      WATCH_TARGET: nut-server:3493
    # HTTP_PORT defaults to 9995
    ports:
      - "9995:9995/tcp"
    # Security hardening
    user: "5000:5000"
    read_only: true
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    mem_limit: 64m
    cpus: 0.5
    pids_limit: 64
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"
```

The image is a single static binary on `scratch` (no shell, no OS), so it runs fine as a non-root user with a read-only filesystem and no Linux capabilities. It needs only a few MB of memory; the limits above leave plenty of headroom.

### Prometheus

Example `prometheus.yml`:

```yaml
global:
    scrape_interval: 15s
    scrape_timeout: 10s

scrape_configs:
  - job_name: "nut"
    static_configs:
      # Insert NUT server address here
      - targets: ["nut-server:3493"]
    relabel_configs:
      - source_labels: [__address__]
        target_label: __param_target
      - source_labels: [__param_target]
        target_label: instance
      - target_label: __address__
        # Insert NUT exporter address here
        replacement: nut-exporter:9995
```

In the above example, `nut-exporter:9995` is the address and port of the NUT _exporter_ while `nut-server:3493` is the address and port of the NUT _server_ to query through the exporter.

### Kubernetes Resource Usage

Example container resources requests and limits.
This was done by scraping one NUT server with two UPSes.
Resource usage was observed across 7 days period.
This can be lowered even more but should be sufficient as a starting point.

```yaml
resources:
  limits:
    cpu: "10m"
    memory: "16Mi"
  requests:
    cpu: "1m"
    memory: "8Mi"
```

### Grafana

Dashboards (identical except for the cost currency): [₪ ILS](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-ils.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-ils.json)) · [$ USD](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-usd.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-usd.json)) · [€ EUR](https://github.com/ShlomiD83/nut-exporter/blob/main/dashboards/nut-exporter-charging-status-eur.json) ([download](https://raw.githubusercontent.com/ShlomiD83/nut-exporter/main/dashboards/nut-exporter-charging-status-eur.json)). Import one in Grafana (Dashboards → New → Import) and pick your Prometheus data source; importing the same file again updates it in place. It shows UPS status (including a *Charging* state when online below 100%), runtime as `mm:ss`, load, battery charge, input/output voltage, frequency and beeper status, power drawn, an outage timeline with outage count and time on battery, and energy and cost for this or last month.

The original upstream dashboard is [14371](https://grafana.com/grafana/dashboards/14371).

**Required plugin:** the UPS runtime is displayed as `mm:ss` with the [Dynamic Text](https://grafana.com/grafana/plugins/marcusolsson-dynamictext-panel/) panel plugin. Grafana installs it on startup when this environment variable is set on the Grafana container:

```
GF_PLUGINS_PREINSTALL=marcusolsson-dynamictext-panel
```

Docker CLI:

```sh
docker run -d -p 3000:3000 -e GF_PLUGINS_PREINSTALL=marcusolsson-dynamictext-panel grafana/grafana
```

Docker Compose:

```yaml
services:
  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_PLUGINS_PREINSTALL=marcusolsson-dynamictext-panel
```

If you add the variable to an existing Grafana container, recreate the container (e.g. `docker compose up -d grafana`) so it picks up the change.

**Dashboard settings** (at the top of the dashboard):

- **Cost period**: *This month* (1st of the month until now) or *Last month* (the whole previous month), for the *Energy* and *Cost* panels. They ignore the dashboard time picker.

- **Price per kWh** (default `0.6352` ₪ / `0.17` $ / `0.30` €): your electricity tariff, used by the *Cost* panel.
- **Discount (%)** (default `7` in the ₪ dashboard, `0` in the others): subtracted from the cost.

To keep your values, save the dashboard with **Update default variable values** ticked.

The *Power*, *Energy* and *Cost* panels need a UPS that reports `ups.realpower`; the outage panels work with any UPS (`ups.status`).

**Catching short power blips:** Prometheus only scrapes every 15–60 s, so an outage of a few seconds can fall between two scrapes. Set `WATCH_TARGET` on the exporter (see below) and it checks NUT every 2 s on its own; the *Outages* and *Time on battery* panels and the red *On battery* markers on the graphs then include even short blips.

The ₪ dashboard is the source; `node manage/make-currency-dashboards.js` regenerates the $ and € ones from it.

## Configuration

### Docker Image Versions

Use `latest` for the newest build, or a date tag (e.g. `2026-11-18`) to pin a specific build. Images are multi-arch (`linux/amd64`, `linux/arm64`); Docker pulls the right one automatically. Both `romekd/prometheus-nut-exporter` and `ghcr.io/shlomid83/nut-exporter` carry the same tags.

### Environment Variables

- `RUST_LOG` (defaults to `info`): The log level used by the console/STDOUT. Set to `debug` so show HTTP requests and `trace` to show extensive debugging info.
- `HTTP_ADDRESS` (defaults to `::`): The HTTP server will listen on this IP. Set to `127.0.0.1` or `::1` to only allow local access.
- `HTTP_PORT` (defaults to `9995`): The HTTP server port.
- `HTTP_PATH` (defaults to `nut`): The HTTP server metrics path. You may want to set it to `/metrics` on new setups to avoid extra Prometheus configuration (not changed here due to compatibility).
- `WATCH_TARGET` (not set by default): NUT server(s) to check every 2 seconds in the background for switches to battery, e.g. `192.168.1.10:3493` (comma-separated for several). Enables the `nut_on_battery_events_total`, `nut_on_battery_seconds_total` and `nut_last_on_battery_timestamp_seconds` metrics, so short outages are counted even when they fall between Prometheus scrapes. Use the same address as the `target` in your Prometheus config.
- `PRINT_METRICS_AND_EXIT` (defaults to `false`): Print a Markdown-formatted table consisting of all metrics and then immediately exit. Used mainly for generating documentation.

## Metrics

See [metrics](metrics.md).

## License

GNU General Public License version 3 (GPLv3).

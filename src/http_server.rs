use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use regex::Regex;
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;

use crate::meta::{APP_NAME, APP_AUTHOR, APP_VERSION};
use crate::common::ErrorResult;
use crate::config::Config;
use crate::nut_client::scrape_nut;
use crate::openmetrics_builder::build_openmetrics_content;
use crate::watcher;

const CONTENT_TYPE_TEXT: &str = "text/plain; charset=UTF-8";
const CONTENT_TYPE_OPENMETRICS: &str = "application/openmetrics-text; version=1.0.0; charset=UTF-8";
const CONTENT_TYPE_OPENMETRICS_BASE: &str = "application/openmetrics-text";

const SCRAPE_TIMEOUT_HEADER: &str = "x-prometheus-scrape-timeout-seconds";
const SCRAPE_BUDGET_MARGIN_SECS: f64 = 0.5;
const MIN_SCRAPE_BUDGET_SECS: f64 = 1.0;
const MAX_SCRAPE_BUDGET_SECS: f64 = 15.0;

type HttpResponse = Response<Full<Bytes>>;

pub async fn run_server(config: Config, mut shutdown_channel: Receiver<bool>) {
    // Bind to endpoint
    let endpoint = SocketAddr::new(config.http_address, config.http_port);
    log::info!("Binding to endpoint: http://{}", endpoint);
    let listener = match TcpListener::bind(endpoint).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Server failed to bind to endpoint: {}", err);
            return;
        },
    };

    // Serve connections (HTTP/1 and HTTP/2) until shutdown is signalled
    let config = Arc::new(config);
    let builder = Arc::new(auto::Builder::new(TokioExecutor::new()));
    loop {
        let (stream, remote_addr) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(err) => {
                    log::error!("Failed to accept connection: {}", err);
                    continue;
                },
            },
            _ = shutdown_channel.recv() => break,
        };

        let config = config.clone();
        let builder = builder.clone();
        tokio::spawn(async move {
            let service = service_fn(move |request: Request<Incoming>| {
                let config = config.clone();
                async move { entrypoint(&config, request, remote_addr).await }
            });
            if let Err(err) = builder.serve_connection(TokioIo::new(stream), service).await {
                log::debug!("Server connection error: {}", err);
            }
        });
    }
}

async fn entrypoint(config: &Config, request: Request<Incoming>, remote_addr: SocketAddr) -> Result<HttpResponse, Infallible> {
    log::trace!("HTTP request from: {}", remote_addr);
    log::trace!("HTTP request URL: {}", request.uri().path());

    let is_method_get = request.method() == Method::GET;
    let path = request.uri().path();
    let response = if path == "/" {
        if is_method_get {
            endpoint_home(config)
        } else {
            endpoint_method_not_allowed()
        }
    } else if path == config.http_path {
        if is_method_get {
            let query = request.uri().query();
            let accept = request.headers().get("accept").and_then(|value| value.to_str().ok());
            let budget = scrape_budget(request.headers().get(SCRAPE_TIMEOUT_HEADER).and_then(|value| value.to_str().ok()));
            endpoint_metrics(config, query, accept, budget).await
        } else {
            endpoint_method_not_allowed()
        }
    } else {
        endpoint_not_found()
    };

    // Log request to console
    log::debug!("Request: {} {} {} {}", remote_addr, request.method(), request.uri().path(), response.status());

    Ok(response)
}

fn response(status: StatusCode, content: impl Into<String>) -> HttpResponse {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(content.into())))
        .unwrap()
}

fn endpoint_home(config: &Config) -> HttpResponse {
    let mut content = String::new();
    let _ = writeln!(content, "{} version {} by {}.", APP_NAME, APP_VERSION, APP_AUTHOR);
    let _ = writeln!(content);
    let _ = writeln!(content, "Usage: {}?target=<target>", config.http_path);

    response(StatusCode::OK, content)
}

fn endpoint_not_found() -> HttpResponse {
    response(StatusCode::NOT_FOUND, "Not found\n")
}

fn endpoint_method_not_allowed() -> HttpResponse {
    response(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed\n")
}

/// Time allowed for the NUT query: Prometheus' scrape timeout minus a margin (so we can still answer with
/// nut_scrape_success 0 before Prometheus gives up), capped to a sane range.
fn scrape_budget(prometheus_timeout: Option<&str>) -> Duration {
    let seconds = prometheus_timeout
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map_or(MAX_SCRAPE_BUDGET_SECS, |timeout| timeout - SCRAPE_BUDGET_MARGIN_SECS);
    Duration::from_secs_f64(seconds.clamp(MIN_SCRAPE_BUDGET_SECS, MAX_SCRAPE_BUDGET_SECS))
}

async fn endpoint_metrics(config: &Config, query: Option<&str>, accept: Option<&str>, budget: Duration) -> HttpResponse {
    // Check for and parse target
    let usage_message = format!("Usage: {}?target=<target>", config.http_path);
    let target = match parse_target(query) {
        Ok(target) => target,
        Err(err) => return response(StatusCode::BAD_REQUEST, format!("{}\n\n{}", err, usage_message)),
    };

    // Try to scrape NUT server. On failure, still answer (with nut_scrape_success 0) so Prometheus keeps the
    // exporter and watcher metrics instead of discarding the whole scrape.
    let scraped = match tokio::time::timeout(budget, scrape_nut(&target)).await {
        Ok(Ok(x)) => Some(x),
        Ok(Err(err)) => {
            log::warn!("Failed to scrape NUT server {}: {}", target, err);
            None
        },
        Err(_) => {
            log::warn!("Failed to scrape NUT server {}: no answer within {:.1}s", target, budget.as_secs_f64());
            None
        },
    };

    // Generate OpenMetrics output (plus background watcher counters, if this target is watched)
    let watched = watcher::snapshot(&target);
    let content = build_openmetrics_content(scraped.as_ref().map(|(upses, version)| (upses, version.as_str())), watched.as_ref());

    // Set content type
    let content_type = match accept {
        Some(accept) if accept.contains(CONTENT_TYPE_OPENMETRICS_BASE) => CONTENT_TYPE_OPENMETRICS,
        _ => CONTENT_TYPE_TEXT,
    };

    let mut ok_response = response(StatusCode::OK, content);
    ok_response.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static(content_type));
    ok_response
}

fn parse_target(query: Option<&str>) -> ErrorResult<String> {
    let query_args: HashMap<String, String> = form_urlencoded::parse(query.unwrap_or("").as_bytes()).into_owned().collect();
    match query_args.get("target") {
        Some(target_raw) => normalize_target(target_raw),
        None => Err("Missing target.".into()),
    }
}

/// Turn "host" or "host:port" into "host:port" (default NUT port if missing).
pub fn normalize_target(target_raw: &str) -> ErrorResult<String> {
    // Match domain, IPv4 address or IPv6 addres, with optional port number
    static TARGET_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^(?P<host>\[[^\]]+\]|[^:]+)(?::(?P<port>[0-9]+))?$"#).unwrap());

    let default_port = Config::DEFAULT_NUT_PORT.to_string();
    let target = match TARGET_PATTERN.captures(target_raw) {
        Some(captures) => {
            let host = captures.name("host").unwrap().as_str();
            let port = match captures.name("port") {
                Some(port) => port.as_str(),
                None => default_port.as_str(),
            };
            format!("{}:{}", host, port)
        },
        None => return Err(format!("Malformed target \"{}\".", target_raw).into()),
    };

    Ok(target)
}

//! Background watcher that polls NUT servers every few seconds and counts switches to battery.
//!
//! Prometheus only scrapes every 15-60 s, so a short power blip (a few seconds on battery) usually
//! falls between two scrapes and is never recorded. The watcher polls on its own schedule and keeps
//! counters that Prometheus can read at its normal pace.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast::Receiver;
use tokio::time::MissedTickBehavior;

use crate::nut_client::scrape_nut;

pub const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Default)]
struct UpsState {
    on_battery_since: Option<Instant>,
    events: u64,
    completed_seconds: f64,
    last_event_unix: Option<f64>,
}

/// Watcher counters for one UPS at the time of a scrape.
#[derive(Debug, Clone)]
pub struct UpsSnapshot {
    pub ups: String,
    pub events: u64,
    pub seconds: f64,
    pub last_event_unix: Option<f64>,
}

#[derive(Debug, Default)]
struct TargetState {
    /// Whether the last poll of this NUT server succeeded.
    up: bool,
    upses: HashMap<String, UpsState>,
}

/// Watcher state for one target at the time of a scrape.
#[derive(Debug, Clone)]
pub struct WatchSnapshot {
    pub up: bool,
    pub upses: Vec<UpsSnapshot>,
}

// Normalized target ("host:port") -> state
static STATE: LazyLock<Mutex<HashMap<String, TargetState>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn run_watcher(target: String, mut shutdown_channel: Receiver<bool>) {
    log::info!("Watching NUT server {} for switches to battery every {}s", target, POLL_INTERVAL.as_secs());
    STATE.lock().unwrap().entry(target.clone()).or_default();
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut failing = false;

    loop {
        tokio::select! {
            _ = interval.tick() => {},
            _ = shutdown_channel.recv() => break,
        }

        match scrape_nut(&target).await {
            Ok((upses, _)) => {
                if failing {
                    log::info!("Watcher reconnected to NUT server {}", target);
                    failing = false;
                }
                let mut state = STATE.lock().unwrap();
                let target_state = state.entry(target.clone()).or_default();
                target_state.up = true;
                for (ups, vars) in upses.iter() {
                    let on_battery = vars.get("ups.status").is_some_and(|status| status.split(' ').any(|s| s == "OB"));
                    update_state(target_state.upses.entry(ups.clone()).or_default(), ups, on_battery);
                }
            },
            Err(err) => {
                // Keep the counters; mark the watcher as down and log once per failure streak
                STATE.lock().unwrap().entry(target.clone()).or_default().up = false;
                if !failing {
                    log::warn!("Watcher failed to poll NUT server {}: {}", target, err);
                    failing = true;
                }
            },
        }
    }
}

fn update_state(state: &mut UpsState, ups: &str, on_battery: bool) {
    match (on_battery, state.on_battery_since) {
        (true, None) => {
            state.events += 1;
            state.on_battery_since = Some(Instant::now());
            state.last_event_unix = Some(unix_now());
            log::info!("UPS {} switched to battery", ups);
        },
        (false, Some(since)) => {
            let seconds = since.elapsed().as_secs_f64();
            state.completed_seconds += seconds;
            state.on_battery_since = None;
            log::info!("UPS {} back on line power after {:.0}s on battery", ups, seconds);
        },
        _ => {},
    }
}

/// Watcher state for the given (normalized) target, or None if it isn't watched.
pub fn snapshot(target: &str) -> Option<WatchSnapshot> {
    let state = STATE.lock().unwrap();
    let target_state = state.get(target)?;
    let mut snapshots: Vec<UpsSnapshot> = target_state.upses.iter().map(|(ups, s)| UpsSnapshot {
        ups: ups.clone(),
        events: s.events,
        seconds: s.completed_seconds + s.on_battery_since.map_or(0.0, |since| since.elapsed().as_secs_f64()),
        last_event_unix: s.last_event_unix,
    }).collect();
    snapshots.sort_by(|a, b| a.ups.cmp(&b.ups));
    Some(WatchSnapshot { up: target_state.up, upses: snapshots })
}

fn unix_now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

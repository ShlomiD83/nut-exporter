//! Monthly energy history.
//!
//! The background watcher feeds every successful poll (ups.realpower) into this module, which adds up
//! kWh per UPS per calendar month (in the configured time zone) and saves the totals to a JSON file.
//! Every month is exported on every scrape, so the history survives any Prometheus retention.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// Polls further apart than this are treated as a gap (exporter or NUT down) and not counted.
const MAX_SAMPLE_GAP: Duration = Duration::from_secs(30);
/// How often the history file is written when it has changed.
const SAVE_INTERVAL: Duration = Duration::from_secs(60);
const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MonthEnergy {
    /// Energy used in the month, in kWh.
    pub kwh: f64,
    /// Time covered by measurements in the month, in seconds.
    pub seconds: f64,
}

/// One row of history for the metrics output.
#[derive(Debug, Clone, PartialEq)]
pub struct MonthRow {
    pub ups: String,
    pub month: String,
    pub energy: MonthEnergy,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryData {
    version: u32,
    /// UPS name -> "YYYY-MM" -> totals
    upses: BTreeMap<String, BTreeMap<String, MonthEnergy>>,
}

struct LastSample {
    at: Instant,
    watts: f64,
}

struct History {
    path: PathBuf,
    time_zone: TimeZone,
    data: HistoryData,
    last_samples: HashMap<String, LastSample>,
    dirty: bool,
    last_save: Instant,
    save_failing: bool,
}

static HISTORY: LazyLock<Mutex<Option<History>>> = LazyLock::new(|| Mutex::new(None));

/// Enable the history, loading the existing file if there is one.
pub fn init(path: &str) {
    let path = PathBuf::from(path);
    let time_zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    let data = load(&path);
    let months: usize = data.upses.values().map(BTreeMap::len).sum();
    log::info!(
        "Monthly energy history in {} (time zone {}, {} stored month(s))",
        path.display(), time_zone.iana_name().unwrap_or("UTC"), months,
    );
    *HISTORY.lock().unwrap() = Some(History {
        path, time_zone, data, last_samples: HashMap::new(), dirty: false, last_save: Instant::now(), save_failing: false,
    });
}

fn load(path: &Path) -> HistoryData {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return HistoryData { version: FILE_VERSION, ..Default::default() },
        Err(err) => {
            log::error!("Failed to read history file {}: {}", path.display(), err);
            return HistoryData { version: FILE_VERSION, ..Default::default() };
        },
    };
    match serde_json::from_str::<HistoryData>(&text) {
        Ok(data) => data,
        Err(err) => {
            // Keep the unreadable file for inspection instead of overwriting it
            let backup = path.with_extension("json.bad");
            log::error!("History file {} is unreadable ({}); moved it to {} and starting fresh", path.display(), err, backup.display());
            let _ = std::fs::rename(path, &backup);
            HistoryData { version: FILE_VERSION, ..Default::default() }
        },
    }
}

/// Feed one power reading for a UPS (called after each successful watcher poll).
pub fn record(ups: &str, watts: f64) {
    let mut guard = HISTORY.lock().unwrap();
    let Some(history) = guard.as_mut() else {
        return;
    };
    let now = Instant::now();
    if let Some(last) = history.last_samples.get(ups) {
        let elapsed = now.duration_since(last.at);
        if elapsed <= MAX_SAMPLE_GAP {
            let month = month_key(Timestamp::now(), &history.time_zone);
            let seconds = elapsed.as_secs_f64();
            // Trapezoid: average of the previous and current reading over the interval
            let kwh = (last.watts + watts) / 2.0 * seconds / 3_600_000.0;
            add_energy(&mut history.data, ups, &month, kwh, seconds);
            history.dirty = true;
        }
    }
    history.last_samples.insert(ups.to_owned(), LastSample { at: now, watts });
}

/// Write the file if it changed and the save interval has passed (or always, when `force`).
pub fn save(force: bool) {
    let mut guard = HISTORY.lock().unwrap();
    let Some(history) = guard.as_mut() else {
        return;
    };
    if !history.dirty || (!force && history.last_save.elapsed() < SAVE_INTERVAL) {
        return;
    }
    history.last_save = Instant::now();
    match write_atomically(&history.path, &history.data) {
        Ok(()) => {
            history.dirty = false;
            if history.save_failing {
                log::info!("History file {} saved again", history.path.display());
                history.save_failing = false;
            }
        },
        Err(err) => {
            if !history.save_failing {
                log::warn!("Failed to save history file {}: {}", history.path.display(), err);
                history.save_failing = true;
            }
        },
    }
}

fn write_atomically(path: &Path, data: &HistoryData) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, json + "\n")?;
    std::fs::rename(&tmp, path)
}

/// All stored months for the given UPSes, oldest first.
pub fn rows(upses: &[String]) -> Vec<MonthRow> {
    let guard = HISTORY.lock().unwrap();
    let Some(history) = guard.as_ref() else {
        return Vec::new();
    };
    upses.iter()
        .filter_map(|ups| history.data.upses.get(ups).map(|months| (ups, months)))
        .flat_map(|(ups, months)| months.iter().map(move |(month, energy)| MonthRow {
            ups: ups.clone(), month: month.clone(), energy: energy.clone(),
        }))
        .collect()
}

/// "YYYY-MM" of the timestamp in the given time zone.
fn month_key(at: Timestamp, time_zone: &TimeZone) -> String {
    at.to_zoned_datetime(time_zone.clone()).strftime("%Y-%m").to_string()
}

fn add_energy(data: &mut HistoryData, ups: &str, month: &str, kwh: f64, seconds: f64) {
    data.version = FILE_VERSION;
    let entry = data.upses.entry(ups.to_owned()).or_default().entry(month.to_owned()).or_default();
    entry.kwh += kwh;
    entry.seconds += seconds;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn month_key_uses_the_time_zone() {
        let jerusalem = TimeZone::get("Asia/Jerusalem").unwrap();
        // 22:30 UTC on 30 Nov is already 00:30 on 1 Dec in Israel (UTC+2 in winter)
        assert_eq!(month_key(ts("2026-11-30T22:30:00Z"), &TimeZone::UTC), "2026-11");
        assert_eq!(month_key(ts("2026-11-30T22:30:00Z"), &jerusalem), "2026-12");
        assert_eq!(month_key(ts("2027-02-28T21:59:59Z"), &jerusalem), "2027-02");
        assert_eq!(month_key(ts("2027-02-28T22:00:00Z"), &jerusalem), "2027-03");
    }

    #[test]
    fn energy_is_split_by_month() {
        let mut data = HistoryData::default();
        add_energy(&mut data, "ups1", "2026-11", 1.5, 3600.0);
        add_energy(&mut data, "ups1", "2026-11", 0.5, 1800.0);
        add_energy(&mut data, "ups1", "2026-12", 0.25, 900.0);
        let months = &data.upses["ups1"];
        assert_eq!(months["2026-11"], MonthEnergy { kwh: 2.0, seconds: 5400.0 });
        assert_eq!(months["2026-12"], MonthEnergy { kwh: 0.25, seconds: 900.0 });
        assert_eq!(data.version, FILE_VERSION);
    }

    #[test]
    fn history_file_round_trips() {
        let mut data = HistoryData::default();
        add_energy(&mut data, "ups1", "2026-11", 12.345, 86400.0);
        let path = std::env::temp_dir().join(format!("nut-history-test-{}.json", std::process::id()));
        write_atomically(&path, &data).unwrap();
        let loaded = load(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(loaded.upses["ups1"]["2026-11"], MonthEnergy { kwh: 12.345, seconds: 86400.0 });
    }
}

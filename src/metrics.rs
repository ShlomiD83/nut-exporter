use std::collections::HashMap;
use std::sync::LazyLock;

pub type VarMap = HashMap<String, String>;
pub type UpsVarMap = HashMap<String, VarMap>;
pub type NutVersion = String;

pub const UPS_DESCRIPTION_PSEUDOVAR: &str = "_description";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VarTransform {
    None,
    Percentage,
    BeeperStatus,
    OldUpsStatus,
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub metric: &'static str,
    pub help: &'static str,
    pub type_: &'static str,
    pub unit: &'static str,
    pub nut_var: &'static str,
    pub var_transform: VarTransform,
    pub is_integer: bool,
}

pub const UPS_STATUS_ELEMENTS: [&str; 11] = [
    "OL",       // online
    "OB",       // on battery
    "LB",       // low battery (critical)
    "CHRG",     // charging
    "RB",       // replace battery
    "FSD",      // forced shutdown
    "BYPASS",   // battery bypass
    "SD",       // shutdown
    "CP",       // cable power
    "BOOST",    // boosted voltage
    "OFF",      // off
];

// Special metrics
pub const EXPORTER_INFO_METRIC: Metric = Metric {
    metric: "nut_exporter_info",
    help: "Metadata about the exporter.",
    type_: "info",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const SERVER_INFO_METRIC: Metric = Metric {
    metric: "nut_server_info",
    help: "Metadata about the NUT server.",
    type_: "info",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const UPS_INFO_METRIC: Metric = Metric {
    metric: "nut_ups_info",
    help: "Metadata about the UPS.",
    type_: "info",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const UPS_STATUS_METRIC: Metric = Metric {
    metric: "nut_ups_status",
    help: "UPS status. Check for a specific status with the \"status\" label.",
    type_: "stateset",
    unit: "",
    nut_var: "ups.status",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const SCRAPE_SUCCESS_METRIC: Metric = Metric {
    metric: "nut_scrape_success",
    help: "1 if the NUT server could be scraped, 0 if not (the UPS metrics are then missing).",
    type_: "gauge",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};
// Background watcher metrics (only present when the target is listed in WATCH_TARGET)
pub const WATCHER_UP_METRIC: Metric = Metric {
    metric: "nut_watcher_up",
    help: "1 if the background watcher's last poll of this NUT server succeeded, 0 if it failed (needs WATCH_TARGET).",
    type_: "gauge",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const ENERGY_MONTH_METRIC: Metric = Metric {
    metric: "nut_energy_month_kwh",
    help: "Energy used per calendar month in kWh, from the background watcher's power readings (needs WATCH_TARGET and HISTORY_FILE).",
    type_: "gauge",
    unit: "",
    nut_var: "ups.realpower",
    var_transform: VarTransform::None,
    is_integer: false,
};
pub const ENERGY_MONTH_SECONDS_METRIC: Metric = Metric {
    metric: "nut_energy_month_seconds",
    help: "Time covered by power readings per calendar month (needs WATCH_TARGET and HISTORY_FILE).",
    type_: "gauge",
    unit: "seconds",
    nut_var: "ups.realpower",
    var_transform: VarTransform::None,
    is_integer: false,
};
pub const ON_BATTERY_EVENTS_METRIC: Metric = Metric {
    metric: "nut_on_battery_events",
    help: "Number of switches to battery seen by the background watcher (polls every 2 s; needs WATCH_TARGET).",
    type_: "counter",
    unit: "",
    nut_var: "ups.status",
    var_transform: VarTransform::None,
    is_integer: true,
};
pub const ON_BATTERY_SECONDS_METRIC: Metric = Metric {
    metric: "nut_on_battery_seconds",
    help: "Total time on battery seen by the background watcher (needs WATCH_TARGET).",
    type_: "counter",
    unit: "seconds",
    nut_var: "ups.status",
    var_transform: VarTransform::None,
    is_integer: false,
};
pub const LAST_ON_BATTERY_METRIC: Metric = Metric {
    metric: "nut_last_on_battery_timestamp_seconds",
    help: "Unix time of the last switch to battery seen by the background watcher (needs WATCH_TARGET).",
    type_: "gauge",
    unit: "seconds",
    nut_var: "ups.status",
    var_transform: VarTransform::None,
    is_integer: false,
};
// Deprecated special metrics
pub const OLD_SERVER_INFO_METRIC: Metric = Metric {
    metric: "nut_info",
    help: "Metadata about the NUT server. (Deprecated, use nut_server_info instead.)",
    type_: "info",
    unit: "",
    nut_var: "",
    var_transform: VarTransform::None,
    is_integer: true,
};

// Basic metrics
pub static BASIC_METRICS: [Metric; 44] = [
    // Status, uptime, load
    Metric {
        metric: "nut_beeper_status",
        help: "If the beeper is enabled. Unknown (0), enabled (1), disabled (2) or muted (3).",
        type_: "gauge",
        unit: "",
        nut_var: "ups.beeper.status",
        var_transform: VarTransform::BeeperStatus,
        is_integer: true,
    },
    Metric {
        metric: "nut_uptime_seconds",
        help: "Device uptime.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "device.uptime",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_load",
        help: "Load. (0-1)",
        type_: "gauge",
        unit: "",
        nut_var: "ups.load",
        var_transform: VarTransform::Percentage,
        is_integer: false,
    },
    Metric {
        metric: "nut_temperature_celsius",
        help: "UPS temperature",
        type_: "gauge",
        unit: "celsius",
        nut_var: "ups.temperature",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    // Battery
    Metric {
        metric: "nut_battery_charge",
        help: "Battery level. (0-1)",
        type_: "gauge",
        unit: "",
        nut_var: "battery.charge",
        var_transform: VarTransform::Percentage,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_charge_low",
        help: "Battery level threshold for low state. (0-1)",
        type_: "gauge",
        unit: "",
        nut_var: "battery.charge.low",
        var_transform: VarTransform::Percentage,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_charge_warning",
        help: "Battery level threshold for warning state. (0-1)",
        type_: "gauge",
        unit: "",
        nut_var: "battery.charge.warning",
        var_transform: VarTransform::Percentage,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_charge_restart",
        help: "Battery level threshold for restarting after power-off. (0-1)",
        type_: "gauge",
        unit: "",
        nut_var: "battery.charge.restart",
        var_transform: VarTransform::Percentage,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_runtime_seconds",
        help: "Battery runtime.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "battery.runtime",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_battery_runtime_low_seconds",
        help: "Battery runtime threshold for state low.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "battery.runtime.low",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_battery_runtime_restart_seconds",
        help: "Battery runtime threshold for restart after power-off.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "battery.runtime.restart",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_delay_shutdown_seconds",
        help: "Interval to wait after shutdown with delay command.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "ups.delay.shutdown",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_delay_start_seconds",
        help: "Interval to wait before (re)starting the load.",
        type_: "gauge",
        unit: "seconds",
        nut_var: "ups.delay.start",
        var_transform: VarTransform::None,
        is_integer: true,
    },
    Metric {
        metric: "nut_battery_voltage_volts",
        help: "Battery voltage.",
        type_: "gauge",
        unit: "volts",
        nut_var: "battery.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_voltage_nominal_volts",
        help: "Battery voltage (nominal).",
        type_: "gauge",
        unit: "volts",
        nut_var: "battery.voltage.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_voltage_high_volts",
        help: "Battery voltage for full (charge level calculation).",
        type_: "gauge",
        unit: "volts",
        nut_var: "battery.voltage.high",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_voltage_low_volts",
        help: "Battery voltage for empty (charge level calculation).",
        type_: "gauge",
        unit: "volts",
        nut_var: "battery.voltage.low",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_battery_temperature_celsius",
        help: "Battery temperature.",
        type_: "gauge",
        unit: "celsius",
        nut_var: "battery.temperature",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    // Input
    Metric {
        metric: "nut_input_voltage_volts",
        help: "Input voltage.",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_voltage_nominal_volts",
        help: "Input voltage (nominal).",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.voltage.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_voltage_minimum_volts",
        help: "Input voltage (minimum seen).",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.voltage.minimum",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_voltage_maximum_volts",
        help: "Input voltage (maximum seen).",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.voltage.maximum",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_transfer_low_volts",
        help: "Input lower transfer threshold.",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.transfer.low",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_transfer_high_volts",
        help: "Input upper transfer threshold.",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.transfer.high",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_current_amperes",
        help: "Input current.",
        type_: "gauge",
        unit: "amperes",
        nut_var: "input.current",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_current_nominal_amperes",
        help: "Input current (nominal).",
        type_: "gauge",
        unit: "amperes",
        nut_var: "input.current.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_frequency_hertz",
        help: "Input frequency.",
        type_: "gauge",
        unit: "hertz",
        nut_var: "input.frequency",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_frequency_nominal_hertz",
        help: "Input frequency (nominal).",
        type_: "gauge",
        unit: "hertz",
        nut_var: "input.frequency.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_frequency_low_hertz",
        help: "Input frequency (low).",
        type_: "gauge",
        unit: "hertz",
        nut_var: "input.frequency.low",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_frequency_high_hertz",
        help: "Input frequency (high).",
        type_: "gauge",
        unit: "hertz",
        nut_var: "input.frequency.high",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    // Output
    Metric {
        metric: "nut_output_voltage_volts",
        help: "Output voltage.",
        type_: "gauge",
        unit: "volts",
        nut_var: "output.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_voltage_nominal_volts",
        help: "Output voltage (nominal).",
        type_: "gauge",
        unit: "volts",
        nut_var: "output.voltage.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_current_amperes",
        help: "Output current.",
        type_: "gauge",
        unit: "amperes",
        nut_var: "output.current",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_current_nominal_amperes",
        help: "Output current (nominal).",
        type_: "gauge",
        unit: "amperes",
        nut_var: "output.current.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_frequency_hertz",
        help: "Output frequency.",
        type_: "gauge",
        unit: "hertz",
        nut_var: "output.frequency",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_frequency_nominal_hertz",
        help: "Output frequency (nominal).",
        type_: "gauge",
        unit: "hertz",
        nut_var: "output.frequency.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    // Power
    Metric {
        metric: "nut_power_watts",
        help: "Apparent power.",
        type_: "gauge",
        unit: "watts",
        nut_var: "ups.power",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_power_nominal_watts",
        help: "Apparent power (nominal).",
        type_: "gauge",
        unit: "watts",
        nut_var: "ups.power.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_real_power_watts",
        help: "Real power.",
        type_: "gauge",
        unit: "watts",
        nut_var: "ups.realpower",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_real_power_nominal_watts",
        help: "Real power (nominal).",
        type_: "gauge",
        unit: "watts",
        nut_var: "ups.realpower.nominal",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    // Deprecated
    Metric {
        metric: "nut_status",
        help: "UPS status. Unknown (0), on line (1, \"OL\"), on battery (2, \"OB\"), or low battery (3, \"LB\"). (Deprecated, use nut_ups_status instead.)",
        type_: "gauge",
        unit: "",
        nut_var: "ups.status",
        var_transform: VarTransform::OldUpsStatus,
        is_integer: true,
    },
    Metric {
        metric: "nut_battery_volts",
        help: "Battery voltage. (Deprecated, use nut_battery_voltage_volts instead.)",
        type_: "gauge",
        unit: "volts",
        nut_var: "battery.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_input_volts",
        help: "Input voltage. (Deprecated, use nut_input_voltage_volts instead.)",
        type_: "gauge",
        unit: "volts",
        nut_var: "input.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
    Metric {
        metric: "nut_output_volts",
        help: "Output voltage. (Deprecated, use nut_output_voltage_volts instead.)",
        type_: "gauge",
        unit: "volts",
        nut_var: "output.voltage",
        var_transform: VarTransform::None,
        is_integer: false,
    },
];

// Contains all metrics names, in insertion order
pub static METRIC_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut vec: Vec<&'static str> = Vec::new();
    vec.push(EXPORTER_INFO_METRIC.metric);
    vec.push(SERVER_INFO_METRIC.metric);
    vec.push(UPS_INFO_METRIC.metric);
    vec.push(OLD_SERVER_INFO_METRIC.metric);
    vec.push(UPS_STATUS_METRIC.metric);
    vec.push(SCRAPE_SUCCESS_METRIC.metric);
    vec.push(WATCHER_UP_METRIC.metric);
    vec.push(ON_BATTERY_EVENTS_METRIC.metric);
    vec.push(ON_BATTERY_SECONDS_METRIC.metric);
    vec.push(LAST_ON_BATTERY_METRIC.metric);
    vec.push(ENERGY_MONTH_METRIC.metric);
    vec.push(ENERGY_MONTH_SECONDS_METRIC.metric);
    for metric in BASIC_METRICS.iter() {
        vec.push(metric.metric);
    }
    vec
});

// Contains all metrics, indexed by metric name
pub static METRICS: LazyLock<HashMap<&'static str, &'static Metric>> = LazyLock::new(|| {
    let mut map: HashMap<&'static str, &'static Metric> = HashMap::new();
    map.insert(EXPORTER_INFO_METRIC.metric, &EXPORTER_INFO_METRIC);
    map.insert(SERVER_INFO_METRIC.metric, &SERVER_INFO_METRIC);
    map.insert(UPS_INFO_METRIC.metric, &UPS_INFO_METRIC);
    map.insert(OLD_SERVER_INFO_METRIC.metric, &OLD_SERVER_INFO_METRIC);
    map.insert(UPS_STATUS_METRIC.metric, &UPS_STATUS_METRIC);
    map.insert(SCRAPE_SUCCESS_METRIC.metric, &SCRAPE_SUCCESS_METRIC);
    map.insert(WATCHER_UP_METRIC.metric, &WATCHER_UP_METRIC);
    map.insert(ON_BATTERY_EVENTS_METRIC.metric, &ON_BATTERY_EVENTS_METRIC);
    map.insert(ON_BATTERY_SECONDS_METRIC.metric, &ON_BATTERY_SECONDS_METRIC);
    map.insert(LAST_ON_BATTERY_METRIC.metric, &LAST_ON_BATTERY_METRIC);
    map.insert(ENERGY_MONTH_METRIC.metric, &ENERGY_MONTH_METRIC);
    map.insert(ENERGY_MONTH_SECONDS_METRIC.metric, &ENERGY_MONTH_SECONDS_METRIC);
    for metric in BASIC_METRICS.iter() {
        map.insert(metric.metric, metric);
    }
    map
});

// Contains all metrics based on NUT vars, indexed by var
pub static VAR_METRICS: LazyLock<HashMap<&'static str, Vec<&'static Metric>>> = LazyLock::new(|| {
    let mut map: HashMap<&'static str, Vec<&'static Metric>> = HashMap::new();
    for metric in BASIC_METRICS.iter() {
        map.entry(metric.nut_var).or_default().push(metric);
    }
    map
});

// Print metrics as Markdown table.
pub fn print_metrics() {
    println!("| Metric | NUT Var | Unit | Description |");
    println!("| - | - | - | - |");
    let print_metric = |metric: &Metric| {
        let row = format!("| `{}` | `{}` | `{}` | {} |", metric.metric, metric.nut_var, metric.unit, metric.help).replace("``", "");
        println!("{}", row)
    };

    print_metric(&EXPORTER_INFO_METRIC);
    print_metric(&SERVER_INFO_METRIC);
    print_metric(&UPS_INFO_METRIC);
    print_metric(&OLD_SERVER_INFO_METRIC);
    print_metric(&UPS_STATUS_METRIC);
    print_metric(&SCRAPE_SUCCESS_METRIC);
    print_metric(&WATCHER_UP_METRIC);
    print_metric(&ON_BATTERY_EVENTS_METRIC);
    print_metric(&ON_BATTERY_SECONDS_METRIC);
    print_metric(&LAST_ON_BATTERY_METRIC);
    print_metric(&ENERGY_MONTH_METRIC);
    print_metric(&ENERGY_MONTH_SECONDS_METRIC);
    for metric in BASIC_METRICS.iter() {
        print_metric(metric);
    }
}

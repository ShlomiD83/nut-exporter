#!/bin/bash

set -u -o pipefail

status_file="$(mktemp)"
history_file="$(mktemp -u).json"
echo "OL CHRG" > "$status_file"

# Start mock (its ups.status can be changed through the status file)
MOCK_STATUS_FILE="$status_file" manage/nut-server-mock.py &>/dev/null &
mock_pid=$!

# Start exporter, with the background watcher polling the mock and the monthly energy history enabled
cargo build --quiet || exit 1
function start_exporter {
    WATCH_TARGET=localhost:3493 HISTORY_FILE="$history_file" target/debug/prometheus-nut-exporter &>/dev/null &
    exporter_pid=$!
}
start_exporter

# Teardown on exit
function teardown {
    kill -9 $mock_pid &>/dev/null
    kill -9 $exporter_pid &>/dev/null
    rm -f "$status_file" "$history_file" "$history_file.tmp"
}
trap teardown EXIT

function fail {
    echo "Failure!"
    echo "$1"
    exit 1
}

function scrape {
    curl -sSf "http://localhost:9995/nut?target=localhost:3493"
}

# Wait for startup (and the watcher's first poll)
sleep 3

# Scrape
scrape_content=$(scrape) || fail "Scrape failed."

# Validate result
echo "$scrape_content" | grep -q '^nut_status{ups="alpha"} 1$' || fail "Scraped result contains errors."
echo "$scrape_content" | grep -q '^nut_scrape_success 1$' || fail "Scrape not marked successful."
echo "$scrape_content" | grep -q '^nut_watcher_up 1$' || fail "Watcher is not up."
echo "$scrape_content" | grep -q '^nut_on_battery_events_total{ups="alpha"} 0$' || fail "Watcher counted an outage that didn't happen."

# Simulate a short outage (shorter than a typical Prometheus scrape interval)
echo "OB DISCHRG" > "$status_file"
sleep 4
echo "OL CHRG" > "$status_file"
sleep 3

scrape_content=$(scrape) || fail "Scrape after outage failed."
echo "$scrape_content" | grep -q '^nut_on_battery_events_total{ups="alpha"} 1$' || fail "Watcher did not count the outage."
seconds=$(echo "$scrape_content" | sed -n 's/^nut_on_battery_seconds_total{ups="alpha"} \([0-9.]*\)$/\1/p')
awk -v s="$seconds" 'BEGIN { exit !(s >= 2 && s <= 7) }' || fail "Unexpected time on battery: ${seconds}s (expected ~4s)."
echo "$scrape_content" | grep -q '^nut_last_on_battery_timestamp_seconds{ups="alpha"} ' || fail "Missing last outage timestamp."

# Monthly energy history: the mock draws 120 W, so energy must accumulate, be saved on shutdown and reloaded
month=$(date +%Y-%m)
function month_kwh {
    echo "$1" | grep "^nut_energy_month_kwh{ups=\"alpha\",month=\"$month\"} " | awk '{ print $2 }'
}
scrape_content=$(scrape) || fail "Scrape for energy history failed."
kwh=$(month_kwh "$scrape_content")
awk -v k="$kwh" 'BEGIN { exit !(k > 0) }' || fail "No energy recorded for $month (got '$kwh')."
kill -TERM $exporter_pid &>/dev/null; wait $exporter_pid 2>/dev/null
grep -q '"alpha"' "$history_file" || fail "History file not written on shutdown."
start_exporter
sleep 3
scrape_content=$(scrape) || fail "Scrape after exporter restart failed."
kwh_after=$(month_kwh "$scrape_content")
awk -v a="$kwh_after" -v b="$kwh" 'BEGIN { exit !(a >= b) }' || fail "Energy history lost on restart ($kwh kWh before, '$kwh_after' after)."

# NUT server goes away: the exporter must still answer, flagging the failed scrape and the failing watcher
kill -9 $mock_pid &>/dev/null; wait $mock_pid 2>/dev/null
sleep 3
scrape_content=$(scrape) || fail "Exporter stopped answering when the NUT server was down."
echo "$scrape_content" | grep -q '^nut_scrape_success 0$' || fail "Failed NUT scrape not flagged."
echo "$scrape_content" | grep -q '^nut_watcher_up 0$' || fail "Watcher not flagged as failing."
echo "$scrape_content" | grep -q '^nut_on_battery_events_total{ups="alpha"} ' || fail "Watcher counters lost while NUT was down."
echo "$scrape_content" | grep -q '^nut_energy_month_kwh{ups="alpha",' || fail "Energy history not reported while NUT was down."
echo "$scrape_content" | grep -q '^nut_status{' && fail "UPS metrics present although NUT was down."

echo "Success! (outage counted: ${seconds}s on battery; energy history $kwh_after kWh kept across restart; NUT-down state reported)"

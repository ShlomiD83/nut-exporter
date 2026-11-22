#!/usr/bin/env node
// Generates the USD and EUR dashboards from the ILS one (the source of truth).
// Usage: node manage/make-currency-dashboards.js
const fs = require("fs");
const path = require("path");

const dir = path.join(__dirname, "..", "dashboards");
const base = JSON.parse(fs.readFileSync(path.join(dir, "nut-exporter-charging-status-ils.json"), "utf8"));
const TITLE = "Prometheus NUT Exporter - Charging Status";

const VARIANTS = [
  { code: "usd", symbol: "$", unit: "currencyUSD", price: "0.17", discount: "0" },
  { code: "eur", symbol: "€", unit: "currencyEUR", price: "0.30", discount: "0" },
];

function setTextbox(d, name, value) {
  const v = d.templating.list.find((x) => x.name === name);
  v.query = value;
  v.current = { text: value, value };
  v.options = [{ selected: true, text: value, value }];
}

for (const variant of VARIANTS) {
  const d = JSON.parse(JSON.stringify(base));
  d.uid = `nut-exporter-charging-${variant.code}`;
  d.title = `${TITLE} (${variant.symbol})`;
  setTextbox(d, "kwh_price", variant.price);
  setTextbox(d, "discount_pct", variant.discount);
  for (const p of d.panels) {
    if (p.fieldConfig?.defaults?.unit === "currencyILS") p.fieldConfig.defaults.unit = variant.unit;
    if (p.description) p.description = p.description.replace(/₪/g, variant.symbol);
  }
  const file = path.join(dir, `nut-exporter-charging-status-${variant.code}.json`);
  fs.writeFileSync(file, JSON.stringify(d, null, 2) + "\n");
  console.log(`wrote ${path.relative(process.cwd(), file)}`);
}

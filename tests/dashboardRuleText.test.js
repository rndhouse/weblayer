const assert = require("assert");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const dashboardPath = path.join(__dirname, "..", "weblayer", "src", "api", "dashboard.rs");
const source = fs.readFileSync(dashboardPath, "utf8");
const start = source.indexOf("    function readableCapturedText");
const end = source.indexOf("    function catchItem", start);

assert(start >= 0, "rule dashboard should define readableCapturedText");
assert(end > start, "rule dashboard text helpers should appear before catchItem");

const helperSource = `${source.slice(start, end)}\nthis.readableCapturedText = readableCapturedText;`;
const context = {};
vm.runInNewContext(helperSource, context, { filename: dashboardPath });

const result = context.readableCapturedText(
  "The Bowie is America's knife.2:22 AM · Jun 2, 2026 · 232 Views",
  "@bowie"
);

assert.strictEqual(result.value, "The Bowie is America's knife.");
assert.strictEqual(result.changed, true);

console.log("dashboard rule text tests passed");

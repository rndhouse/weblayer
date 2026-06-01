const assert = require("assert");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

class FakeElement {
  constructor(tagName, attributes = {}, layout = {}) {
    this.tagName = tagName.toUpperCase();
    this.attributes = { ...attributes };
    this.children = [];
    this.parentElement = null;
    this.style = {
      display: layout.display || "block",
      visibility: layout.visibility || "visible",
      opacity: layout.opacity || "1"
    };
    this.width = layout.width === undefined ? 320 : layout.width;
    this.height = layout.height === undefined ? 80 : layout.height;
  }

  append(...children) {
    for (const child of children) {
      child.parentElement = this;
      this.children.push(child);
    }
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null;
  }

  querySelectorAll(selector) {
    const selectors = selector.split(",").map((value) => value.trim());
    const matches = [];

    const visit = (element) => {
      for (const child of element.children) {
        if (selectors.some((singleSelector) => child.matches(singleSelector))) {
          matches.push(child);
        }
        visit(child);
      }
    };

    visit(this);
    return matches;
  }

  matches(selector) {
    if (selector === "aside") {
      return this.tagName === "ASIDE";
    }
    if (selector === "[role='complementary']") {
      return this.attributes.role === "complementary";
    }
    if (selector === "[data-testid='sidebarColumn']") {
      return this.attributes["data-testid"] === "sidebarColumn";
    }
    if (selector === "[data-weblayer-ui='true']") {
      return this.attributes["data-weblayer-ui"] === "true";
    }
    if (selector === "[aria-label^='Timeline:']") {
      return String(this.attributes["aria-label"] || "").startsWith("Timeline:");
    }

    return false;
  }

  getBoundingClientRect() {
    return {
      width: this.width,
      height: this.height
    };
  }
}

function loadAdapters(root) {
  const window = {
    location: { href: "https://x.com/home" }
  };
  const context = {
    Element: FakeElement,
    URL,
    document: root,
    getComputedStyle: (element) => element.style,
    window
  };
  context.window.window = window;
  context.window.document = root;
  context.window.Element = FakeElement;
  context.window.URL = URL;
  context.window.getComputedStyle = context.getComputedStyle;

  const scriptPath = path.join(__dirname, "..", "shared", "siteAdapters.js");
  vm.runInNewContext(fs.readFileSync(scriptPath, "utf8"), context, {
    filename: scriptPath
  });

  return context.window.WebLayerSiteAdapters;
}

function testXDebugStatsMountPrefersSidebarTimeline() {
  const root = new FakeElement("document");
  const sidebar = new FakeElement("div", { "data-testid": "sidebarColumn" }, {
    width: 360,
    height: 800
  });
  const searchShell = new FakeElement("div", {}, {
    width: 360,
    height: 44
  });
  const timeline = new FakeElement("section", { "aria-label": "Timeline: Sidebar" }, {
    width: 360,
    height: 600
  });

  sidebar.append(searchShell, timeline);
  root.append(sidebar);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/home" }, root);

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(
    context.debugStatsMount(),
    timeline,
    "debug stats should mount in the sidebar timeline, not the clipped search shell"
  );
}

function testXDebugStatsMountFallsBackToVisibleSidebarChild() {
  const root = new FakeElement("document");
  const sidebar = new FakeElement("div", { "data-testid": "sidebarColumn" }, {
    width: 360,
    height: 800
  });
  const moduleShell = new FakeElement("div", {}, {
    width: 360,
    height: 320
  });

  sidebar.append(moduleShell);
  root.append(sidebar);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/home" }, root);

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(
    context.debugStatsMount(),
    moduleShell,
    "debug stats should still mount in a visible sidebar child when no timeline exists"
  );
}

testXDebugStatsMountPrefersSidebarTimeline();
testXDebugStatsMountFallsBackToVisibleSidebarChild();
console.log("x debug stats mount tests passed");

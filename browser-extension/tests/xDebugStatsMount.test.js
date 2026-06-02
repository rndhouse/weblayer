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
    this.innerText = layout.text || "";
    this.textContent = layout.text || "";
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
    if (selector === "main") {
      return this.tagName === "MAIN";
    }
    if (selector === "article[data-testid='tweet']") {
      return this.tagName === "ARTICLE" && this.attributes["data-testid"] === "tweet";
    }
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
    if (selector === "[data-testid='tweetText']") {
      return this.attributes["data-testid"] === "tweetText";
    }
    if (selector === "[aria-label^='Timeline:']") {
      return String(this.attributes["aria-label"] || "").startsWith("Timeline:");
    }

    return false;
  }

  closest(selector) {
    let element = this;
    while (element) {
      if (element.matches(selector)) {
        return element;
      }
      element = element.parentElement;
    }

    return null;
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

function loadBackground() {
  const context = {
    AbortController,
    URL,
    WebSocket: function WebSocket() {},
    chrome: {
      runtime: {
        onMessage: {
          addListener() {}
        }
      },
      storage: {
        local: {
          get(_defaults, callback) {
            callback({});
          }
        }
      }
    },
    clearTimeout,
    fetch,
    setTimeout
  };
  const scriptPath = path.join(__dirname, "..", "shared", "background.js");
  vm.runInNewContext(
    `${fs.readFileSync(scriptPath, "utf8")}\nthis.__normalizeElement = normalizeElement;`,
    context,
    { filename: scriptPath }
  );

  return context;
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

function testXMetadataIncludesTweetBodyText() {
  const root = new FakeElement("document");
  const main = new FakeElement("main");
  const article = new FakeElement("article", { "data-testid": "tweet" }, {
    width: 500,
    height: 160
  });
  const tweetText = new FakeElement("div", { "data-testid": "tweetText" }, {
    text: "Anyone who used a computer between 1985-2010. What game?"
  });

  article.append(tweetText);
  main.append(article);
  root.append(main);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/home" }, root);
  const metadata = context.metadataForElement(article, {
    text: "@alice May 31Anyone who used a computer between 1985-2010. What game?25K4K",
    links: [{ href: "https://x.com/alice/status/12345" }]
  });

  assert.strictEqual(
    metadata.xCom.postText,
    "Anyone who used a computer between 1985-2010. What game?"
  );
}

function testXMetadataDoesNotUseArticleTextAsPostBody() {
  const root = new FakeElement("document");
  const main = new FakeElement("main");
  const article = new FakeElement("article", { "data-testid": "tweet" }, {
    width: 500,
    height: 160,
    text: "The Bowie is America's knife.2:22 AM · Jun 2, 2026 · 232 Views"
  });

  main.append(article);
  root.append(main);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/bowie/status/24680" }, root);
  const metadata = context.metadataForElement(article, {
    text: "The Bowie is America's knife.2:22 AM · Jun 2, 2026 · 232 Views",
    links: [{ href: "https://x.com/bowie/status/24680" }]
  });

  assert.strictEqual(metadata.xCom.postText, null);
}

function testBackgroundPreservesElementMetadata() {
  const background = loadBackground();
  const normalized = background.__normalizeElement({
    clientId: "client-1",
    metadata: {
      xCom: {
        postText: "Body text"
      }
    }
  });

  assert.deepStrictEqual(normalized.metadata, {
    xCom: {
      postText: "Body text"
    }
  });
}

testXDebugStatsMountPrefersSidebarTimeline();
testXDebugStatsMountFallsBackToVisibleSidebarChild();
testXMetadataIncludesTweetBodyText();
testXMetadataDoesNotUseArticleTextAsPostBody();
testBackgroundPreservesElementMetadata();
console.log("x debug stats mount tests passed");

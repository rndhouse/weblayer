const assert = require("assert");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

class FakeElement {
  constructor(tagName, attributes = {}, layout = {}) {
    this.tagName = tagName.toUpperCase();
    this.attributes = { ...attributes };
    this.dataset = {};
    this.children = [];
    this.parentElement = null;
    this.className = "";
    this.style = {
      display: layout.display || "block",
      visibility: layout.visibility || "visible",
      opacity: layout.opacity || "1",
      borderTopWidth: layout.borderTopWidth || "0px",
      borderRightWidth: layout.borderRightWidth || "0px",
      borderBottomWidth: layout.borderBottomWidth || "0px",
      borderLeftWidth: layout.borderLeftWidth || "0px",
      borderTopLeftRadius: layout.borderTopLeftRadius || "0px",
      borderTopRightRadius: layout.borderTopRightRadius || "0px",
      borderBottomRightRadius: layout.borderBottomRightRadius || "0px",
      borderBottomLeftRadius: layout.borderBottomLeftRadius || "0px"
    };
    this.width = layout.width === undefined ? 320 : layout.width;
    this.height = layout.height === undefined ? 80 : layout.height;
    this.innerText = layout.text || "";
    this.textContent = layout.text || "";
    this.classList = {
      add: (...names) => {
        const classNames = this.classNames();
        for (const name of names) {
          classNames.add(name);
        }
        this.className = Array.from(classNames).join(" ");
      },
      remove: (...names) => {
        const classNames = this.classNames();
        for (const name of names) {
          classNames.delete(name);
        }
        this.className = Array.from(classNames).join(" ");
      },
      toggle: (name, force) => {
        const classNames = this.classNames();
        const shouldAdd = force === undefined ? !classNames.has(name) : force;
        if (shouldAdd) {
          classNames.add(name);
        } else {
          classNames.delete(name);
        }
        this.className = Array.from(classNames).join(" ");
        return shouldAdd;
      },
      contains: (name) => this.classNames().has(name)
    };
  }

  append(...children) {
    for (const child of children) {
      this.insertChildAt(child, this.children.length);
    }
  }

  prepend(...children) {
    children.forEach((child, index) => {
      this.insertChildAt(child, index);
    });
  }

  insertBefore(child, reference) {
    const index = this.children.indexOf(reference);
    this.insertChildAt(child, index >= 0 ? index : this.children.length);
  }

  replaceChildren(...children) {
    for (const child of this.children) {
      child.parentElement = null;
    }
    this.children = [];
    this.append(...children);
  }

  remove() {
    if (!this.parentElement) {
      return;
    }

    const siblings = this.parentElement.children;
    const index = siblings.indexOf(this);
    if (index >= 0) {
      siblings.splice(index, 1);
    }
    this.parentElement = null;
  }

  insertChildAt(child, index) {
    if (child.parentElement) {
      const siblings = child.parentElement.children;
      const existingIndex = siblings.indexOf(child);
      if (existingIndex >= 0) {
        siblings.splice(existingIndex, 1);
      }
    }

    child.parentElement = this;
    this.children.splice(index, 0, child);
  }

  contains(target) {
    let element = target;
    while (element) {
      if (element === this) {
        return true;
      }
      element = element.parentElement;
    }

    return false;
  }

  get parentNode() {
    return this.parentElement;
  }

  get nextElementSibling() {
    if (!this.parentElement) {
      return null;
    }

    const siblings = this.parentElement.children;
    const index = siblings.indexOf(this);
    return index >= 0 ? siblings[index + 1] || null : null;
  }

  classNames() {
    return new Set(String(this.className || "").split(/\s+/).filter(Boolean));
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
    const selectors = selector.split(",").map((value) => value.trim());
    if (selectors.length > 1) {
      return selectors.some((singleSelector) => this.matches(singleSelector));
    }

    const classAttributeMatch = selector.match(
      /^\.([A-Za-z0-9_-]+)\[([^\]=]+)=['"]?([^'"]+)['"]?\]$/
    );
    if (classAttributeMatch) {
      return (
        this.classList.contains(classAttributeMatch[1]) &&
        this.attributeValue(classAttributeMatch[2]) === classAttributeMatch[3]
      );
    }

    const classMatch = selector.match(/^\.([A-Za-z0-9_-]+)$/);
    if (classMatch) {
      return this.classList.contains(classMatch[1]);
    }

    const prefixAttributeMatch = selector.match(/^\[([^\]=^]+)\^=['"]?([^'"]+)['"]?\]$/);
    if (prefixAttributeMatch) {
      return String(this.attributeValue(prefixAttributeMatch[1]) || "").startsWith(
        prefixAttributeMatch[2]
      );
    }

    const attributeMatch = selector.match(/^\[([^\]=]+)=['"]?([^'"]+)['"]?\]$/);
    if (attributeMatch) {
      return this.attributeValue(attributeMatch[1]) === attributeMatch[2];
    }

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

  attributeValue(name) {
    if (Object.prototype.hasOwnProperty.call(this.attributes, name)) {
      return this.attributes[name];
    }

    if (name.startsWith("data-")) {
      const datasetKey = name.slice(5).replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
      return this.dataset[datasetKey];
    }

    return undefined;
  }

  getAttribute(name) {
    return this.attributeValue(name) || null;
  }

  setAttribute(name, value) {
    this.attributes[name] = String(value);
  }

  getBoundingClientRect() {
    return {
      width: this.width,
      height: this.height
    };
  }
}

class FakeDocument extends FakeElement {
  constructor() {
    super("document");
    this.documentElement = this;
  }

  createElement(tagName) {
    return new FakeElement(tagName);
  }

  addEventListener() {}
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
    [
      fs.readFileSync(scriptPath, "utf8"),
      "this.__normalizeElement = normalizeElement;",
      "this.__normalizeViewportExposure = normalizeViewportExposure;",
      "this.__normalizeDebugStats = normalizeDebugStats;"
    ].join("\n"),
    context,
    { filename: scriptPath }
  );

  return context;
}

function loadContentScript(root, href = "https://x.com/home") {
  const window = {
    CSS: {
      escape(value) {
        return String(value).replace(/[^a-zA-Z0-9_-]/g, "\\$&");
      }
    },
    location: { href },
    WebLayerSiteAdapters: null
  };
  const context = {
    Element: FakeElement,
    URL,
    chrome: {
      runtime: {
        onMessage: {
          addListener() {}
        }
      }
    },
    document: root,
    getComputedStyle: (element) => element.style,
    location: window.location,
    MutationObserver: class {
      observe() {}
    },
    setTimeout: () => 1,
    window
  };
  context.window.window = window;
  context.window.document = root;
  context.window.Element = FakeElement;
  context.window.URL = URL;
  context.window.getComputedStyle = context.getComputedStyle;

  const scriptPath = path.join(__dirname, "..", "shared", "contentScript.js");
  vm.runInNewContext(
    [
      fs.readFileSync(scriptPath, "utf8"),
      "this.__hideContainerForElement = hideContainerForElement;",
      "this.__collapseHiddenElement = collapseHiddenElement;",
      "this.__hiddenElementForToggle = hiddenElementForToggle;",
      "this.__toggleHiddenElementExpanded = toggleHiddenElementExpanded;"
    ].join("\n"),
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
  const mount = context.debugStatsMount();

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(mount.element, timeline);
  assert.strictEqual(
    mount.placement,
    "before",
    "debug stats should insert before the sidebar timeline, not inside a sidebar module"
  );
}

function testXDebugStatsMountTargetsSidebarModuleAroundTimeline() {
  const root = new FakeElement("document");
  const sidebar = new FakeElement("div", { "data-testid": "sidebarColumn" }, {
    width: 360,
    height: 800
  });
  const searchShell = new FakeElement("div", {}, {
    width: 360,
    height: 44
  });
  const moduleShell = new FakeElement("section", {}, {
    width: 360,
    height: 600,
    borderTopWidth: "1px",
    borderRightWidth: "1px",
    borderBottomWidth: "1px",
    borderLeftWidth: "1px",
    borderTopLeftRadius: "16px"
  });
  const timeline = new FakeElement("div", { "aria-label": "Timeline: What's happening" }, {
    width: 360,
    height: 560
  });

  moduleShell.append(timeline);
  sidebar.append(searchShell, moduleShell);
  root.append(sidebar);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/home" }, root);
  const mount = context.debugStatsMount();

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(
    mount.element,
    moduleShell,
    "debug stats should target the module wrapper, not the nested timeline"
  );
  assert.strictEqual(
    mount.placement,
    "before",
    "debug stats should insert before the bordered sidebar module"
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
  const mount = context.debugStatsMount();

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(mount.element, moduleShell);
  assert.strictEqual(
    mount.placement,
    "before",
    "debug stats should insert before a visible sidebar child when no timeline exists"
  );
}

function testXDebugStatsMountPrependsToEmptySidebar() {
  const root = new FakeElement("document");
  const sidebar = new FakeElement("div", { "data-testid": "sidebarColumn" }, {
    width: 360,
    height: 800
  });

  root.append(sidebar);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/home" }, root);
  const mount = context.debugStatsMount();

  assert(context, "x.com adapter should be active on the home timeline");
  assert.strictEqual(mount.element, sidebar);
  assert.strictEqual(
    mount.placement,
    "prepend",
    "debug stats should prepend to the sidebar only when there is no visible content module"
  );
}

function testXDebugStatsMountSupportsProfileTimeline() {
  const root = new FakeElement("document");
  const sidebar = new FakeElement("div", { "data-testid": "sidebarColumn" }, {
    width: 360,
    height: 800
  });
  const moduleShell = new FakeElement("section", {}, {
    width: 360,
    height: 420,
    borderTopWidth: "1px",
    borderRightWidth: "1px",
    borderBottomWidth: "1px",
    borderLeftWidth: "1px",
    borderTopLeftRadius: "16px"
  });
  const timeline = new FakeElement("div", { "aria-label": "Timeline: You might like" }, {
    width: 360,
    height: 380
  });

  moduleShell.append(timeline);
  sidebar.append(moduleShell);
  root.append(sidebar);

  const adapters = loadAdapters(root);
  const context = adapters.current({ href: "https://x.com/rndhouse" }, root);
  const mount = context && context.debugStatsMount();

  assert(context, "x.com adapter should be active on profile timelines");
  assert.strictEqual(context.pageKind, "profileTimeline");
  assert.strictEqual(
    mount.element,
    moduleShell,
    "profile pages should mount stats before the sidebar module instead of using overlay fallback"
  );
  assert.strictEqual(mount.placement, "before");
}

function testXAdapterDoesNotTreatReservedRouteAsProfile() {
  const root = new FakeElement("document");
  const adapters = loadAdapters(root);

  assert.strictEqual(
    adapters.current({ href: "https://x.com/notifications" }, root),
    null,
    "reserved X routes should not be treated as profile timelines"
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

function testBackgroundUsesSiteScopedDashboardUrl() {
  const background = loadBackground();
  const normalized = background.__normalizeDebugStats(
    {
      site: "x.com",
      sections: []
    },
    "http://127.0.0.1:17891"
  );

  assert.strictEqual(normalized.dashboardUrl, "http://127.0.0.1:17891/x.com/dashboard");
}

function testViewportExposureBridgeExists() {
  const background = loadBackground();
  const contentScript = fs.readFileSync(
    path.join(__dirname, "..", "shared", "contentScript.js"),
    "utf8"
  );
  const normalized = background.__normalizeViewportExposure({
    element: {
      clientId: "client-1",
      text: "Visible post",
      metadata: {
        xCom: {
          postId: "123",
          postText: "Visible post"
        }
      }
    },
    firstVisibleAt: "2026-06-02T13:00:00.000Z",
    lastVisibleAt: "2026-06-02T13:00:02.000Z",
    visibleDurationMs: 2000,
    maxVisibleRatio: 0.75,
    viewportWidth: 1280,
    viewportHeight: 720
  });

  assert.strictEqual(normalized.element.clientId, "client-1");
  assert.strictEqual(normalized.visibleDurationMs, 2000);
  assert.strictEqual(normalized.maxVisibleRatio, 0.75);
  assert.strictEqual(normalized.viewportWidth, 1280);
  assert.strictEqual(normalized.viewportHeight, 720);
  assert(
    contentScript.includes("new IntersectionObserver(handleViewportIntersections") &&
      contentScript.includes('type: "weblayer:viewportExposures"') &&
      contentScript.includes("VIEWPORT_EXPOSURE_MIN_VISIBLE_RATIO = 0.5") &&
      contentScript.includes("VIEWPORT_EXPOSURE_MIN_VISIBLE_MS = 750"),
    "content script should track meaningful viewport exposure and send exposure batches"
  );
}

function testFeedbackPanelStacksReasonAndSaveStatus() {
  const contentScript = fs.readFileSync(
    path.join(__dirname, "..", "shared", "contentScript.js"),
    "utf8"
  );
  const contentCss = fs.readFileSync(
    path.join(__dirname, "..", "shared", "content.css"),
    "utf8"
  );

  assert(
    contentScript.includes('heading.className = "weblayer-feedback-panel-heading"'),
    "feedback panel should group the Reason label and save status in one stacked heading"
  );
  assert(
    contentScript.includes("heading.append(label, status);"),
    "feedback panel heading should place the save status under the Reason label"
  );
  assert(
    contentCss.includes(".weblayer-feedback-panel-heading"),
    "feedback panel heading should have CSS for stacked layout"
  );
}

function testFeedbackButtonAvoidsVerticalLayoutShift() {
  const contentCss = fs.readFileSync(
    path.join(__dirname, "..", "shared", "content.css"),
    "utf8"
  );

  assert(
    !contentCss.includes("translateY("),
    "feedback button should not use vertical transforms that can make the action row jump"
  );
  assert(
    contentCss.includes("flex: 0 0 38px;") &&
      contentCss.includes("max-height: 0;") &&
      contentCss.includes("overflow: visible;"),
    "feedback slot should reserve compact horizontal room without contributing action-row height"
  );
  assert(
    contentCss.includes("position: absolute;") &&
      contentCss.includes("top: -16px;") &&
      contentCss.includes("left: 0;"),
    "feedback button should be positioned inside the zero-height slot"
  );
}

function testHiddenPostsUseExpandablePlaceholder() {
  const contentScript = fs.readFileSync(
    path.join(__dirname, "..", "shared", "contentScript.js"),
    "utf8"
  );
  const contentCss = fs.readFileSync(
    path.join(__dirname, "..", "shared", "content.css"),
    "utf8"
  );

  assert(
    contentCss.includes('.weblayer-hidden:not(.weblayer-hidden--expanded):not([data-weblayer-hidden-mode="inline"])') &&
      contentCss.includes('.weblayer-hidden[data-weblayer-hidden-mode="inline"]:not(.weblayer-hidden--expanded)') &&
      contentCss.includes("> :not(.weblayer-hidden-placeholder)") &&
      contentCss.includes("overflow: visible !important;"),
    "X hidden posts should keep the article mounted and hide only non-placeholder children while collapsed"
  );
  assert(
    contentCss.includes(".weblayer-hidden-placeholder") &&
      contentCss.includes(".weblayer-hidden-placeholder--expanded") &&
      contentCss.includes(".weblayer-hidden-toggle"),
    "hidden posts should render a compact placeholder row with an expanded state"
  );
  assert(
    contentScript.includes("collapseHiddenElement(element, command") &&
      contentScript.includes("hiddenModeForElement(element)") &&
      contentScript.includes("ensureHiddenPlaceholder(element, hiddenMode)") &&
      contentScript.includes("updateHiddenPlaceholderExpandedState(placeholder, expanded)") &&
      contentScript.includes("hiddenElementForToggle(hiddenToggle)") &&
      contentScript.includes("hideContainerForElement(element)") &&
      contentScript.includes('target.closest(".weblayer-hidden-action")') &&
      contentScript.includes('action.type = "button"') &&
      contentScript.includes('action.setAttribute("aria-expanded"') &&
      contentScript.includes('action.textContent = expanded ? "Hide" : "Show"'),
    "hide commands should create a reversible hidden-post toggle"
  );
}

function testHiddenPostsKeepTimelineCellMounted() {
  const root = new FakeDocument();
  const main = new FakeElement("main");
  const cell = new FakeElement("div", { "data-testid": "cellInnerDiv" });
  const article = new FakeElement("article", { "data-testid": "tweet" });
  const text = new FakeElement("div", { "data-testid": "tweetText" }, {
    text: "Hidden post body"
  });

  article.append(text);
  cell.append(article);
  main.append(cell);
  root.append(main);

  const contentScript = loadContentScript(root);
  const hiddenElement = contentScript.__hideContainerForElement(article);

  assert.strictEqual(
    hiddenElement,
    article,
    "X posts should hide the tweet article rather than the timeline cell wrapper"
  );

  contentScript.__collapseHiddenElement(hiddenElement, { reason: "Matched rule" });

  assert(
    article.classList.contains("weblayer-hidden"),
    "the X tweet article should receive the hidden state"
  );
  assert(
    !cell.classList.contains("weblayer-hidden"),
    "the X timeline cell should stay mounted and visible"
  );

  const placeholder = article.children[0];
  const action = placeholder.querySelector(".weblayer-hidden-action");
  assert.strictEqual(
    cell.children[0],
    article,
    "the X timeline cell should still contain the tweet article at its original slot"
  );
  assert.strictEqual(
    placeholder.nextElementSibling,
    text,
    "the WebLayer placeholder should sit inside the tweet article before the post content"
  );
  assert.strictEqual(
    cell.parentElement,
    main,
    "the timeline cell should remain in its original timeline position"
  );
  assert.strictEqual(
    article.dataset.weblayerHiddenMode,
    "inline",
    "tweet articles should use inline hidden mode so Show can reveal the article content"
  );
  assert(action, "the placeholder should include a Show/Hide action");

  const resolvedElement = contentScript.__hiddenElementForToggle(action);
  assert.strictEqual(
    resolvedElement,
    article,
    "the placeholder action should resolve back to the hidden tweet article"
  );

  contentScript.__toggleHiddenElementExpanded(resolvedElement);
  assert(
    article.classList.contains("weblayer-hidden--expanded"),
    "Show should expand the hidden tweet article"
  );
  assert(
    placeholder.classList.contains("weblayer-hidden-placeholder--expanded"),
    "Show should mark the placeholder as expanded"
  );
  assert.strictEqual(action.textContent, "Hide");

  contentScript.__toggleHiddenElementExpanded(resolvedElement);
  assert(
    !article.classList.contains("weblayer-hidden--expanded"),
    "Hide should collapse the hidden tweet article again"
  );
  assert.strictEqual(action.textContent, "Show");
}

function testHiddenPostsKeepStatusThreadPlaceholderAtArticleSlot() {
  const root = new FakeDocument();
  const main = new FakeElement("main");
  const threadCell = new FakeElement("div", { "data-testid": "cellInnerDiv" });
  const threadHeader = new FakeElement("div", {}, { text: "Thread header" });
  const article = new FakeElement("article", { "data-testid": "tweet" });
  const text = new FakeElement("div", { "data-testid": "tweetText" }, {
    text: "Hidden status-thread post body"
  });

  article.append(text);
  threadCell.append(threadHeader, article);
  main.append(threadCell);
  root.append(main);

  const contentScript = loadContentScript(
    root,
    "https://x.com/ShortDramaCh/status/2061767682459046133"
  );
  const hiddenElement = contentScript.__hideContainerForElement(article);

  assert.strictEqual(
    hiddenElement,
    article,
    "status-thread posts should hide the article itself so the placeholder stays at the post slot"
  );

  contentScript.__collapseHiddenElement(hiddenElement, { reason: "Matched rule" });

  assert(
    article.classList.contains("weblayer-hidden"),
    "the status-thread article should receive the hidden state"
  );
  assert(
    !threadCell.classList.contains("weblayer-hidden"),
    "the broader status-thread cell should not be hidden"
  );

  const placeholder = article.children[0];
  const action = placeholder.querySelector(".weblayer-hidden-action");
  assert.strictEqual(
    threadCell.children[1],
    article,
    "the status-thread article should remain in its original thread slot"
  );
  assert.strictEqual(
    placeholder.nextElementSibling,
    text,
    "the WebLayer placeholder should sit inside the status-thread article before the post content"
  );

  const resolvedElement = contentScript.__hiddenElementForToggle(action);
  assert.strictEqual(
    resolvedElement,
    article,
    "the placeholder action should resolve back to the hidden status-thread article"
  );

  contentScript.__toggleHiddenElementExpanded(resolvedElement);
  assert(article.classList.contains("weblayer-hidden--expanded"));
  assert.strictEqual(action.textContent, "Hide");
}

testXDebugStatsMountPrefersSidebarTimeline();
testXDebugStatsMountTargetsSidebarModuleAroundTimeline();
testXDebugStatsMountFallsBackToVisibleSidebarChild();
testXDebugStatsMountPrependsToEmptySidebar();
testXDebugStatsMountSupportsProfileTimeline();
testXAdapterDoesNotTreatReservedRouteAsProfile();
testXMetadataIncludesTweetBodyText();
testXMetadataDoesNotUseArticleTextAsPostBody();
testBackgroundPreservesElementMetadata();
testBackgroundUsesSiteScopedDashboardUrl();
testViewportExposureBridgeExists();
testFeedbackPanelStacksReasonAndSaveStatus();
testFeedbackButtonAvoidsVerticalLayoutShift();
testHiddenPostsUseExpandablePlaceholder();
testHiddenPostsKeepTimelineCellMounted();
testHiddenPostsKeepStatusThreadPlaceholderAtArticleSlot();
console.log("x debug stats mount tests passed");

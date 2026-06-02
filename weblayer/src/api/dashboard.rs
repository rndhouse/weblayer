use axum::response::Html;

pub(super) async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

pub(super) async fn rule_dashboard() -> Html<&'static str> {
    Html(RULE_DASHBOARD_HTML)
}

pub(super) async fn proposal_dashboard() -> Html<&'static str> {
    Html(PROPOSAL_DASHBOARD_HTML)
}

pub(super) async fn posts_dashboard() -> Html<&'static str> {
    Html(POSTS_DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>WebLayer Dashboard</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #0f172a;
      --panel: #111827;
      --panel-2: #172033;
      --border: #334155;
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #7dd3fc;
      --ok: #86efac;
      font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
    }

    main {
      width: min(1180px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 28px 0 40px;
    }

    header {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 16px;
      margin-bottom: 18px;
    }

    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: 0;
    }

    h2 {
      margin: 0 0 12px;
      font-size: 15px;
      letter-spacing: 0;
    }

    button {
      font: inherit;
    }

    .meta {
      color: var(--muted);
      font-size: 12px;
      white-space: nowrap;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-bottom: 12px;
    }

    .layout {
      display: grid;
      grid-template-columns: minmax(0, 1.25fr) minmax(320px, 0.75fr);
      gap: 12px;
    }

    .panel, .stat {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
    }

    .panel {
      padding: 14px;
      min-width: 0;
    }

    .panel + .panel {
      margin-top: 12px;
    }

    .panel-heading {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      margin-bottom: 12px;
    }

    .panel-heading h2 {
      margin: 0;
    }

    .action-button {
      flex: 0 0 auto;
      min-height: 30px;
      padding: 0 10px;
      border: 1px solid var(--accent);
      border-radius: 6px;
      background: transparent;
      color: var(--accent);
      cursor: pointer;
      font-weight: 700;
    }

    .action-button:hover,
    .action-button:focus-visible {
      background: rgba(125, 211, 252, 0.12);
      outline: none;
    }

    .action-button:disabled {
      border-color: var(--border);
      color: var(--muted);
      cursor: wait;
    }

    .stat {
      padding: 12px;
    }

    .stat-link {
      display: block;
      color: inherit;
      text-decoration: none;
    }

    .stat-link:hover,
    .stat-link:focus-visible {
      border-color: var(--accent);
      outline: none;
    }

    .stat-label {
      color: var(--muted);
      font-size: 12px;
    }

    .stat-value {
      margin-top: 4px;
      font-size: 24px;
      font-weight: 700;
    }

    .list {
      display: grid;
      gap: 8px;
    }

    .item {
      padding: 10px;
      border: 1px solid rgba(148, 163, 184, 0.22);
      border-radius: 6px;
      background: var(--panel-2);
    }

    .item-link {
      display: block;
      width: 100%;
      color: inherit;
      font: inherit;
      text-align: left;
      text-decoration: none;
    }

    .item-link:hover,
    .item-link:focus-visible {
      border-color: var(--accent);
      outline: none;
    }

    .item-title {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      font-weight: 700;
    }

    .pill {
      flex: 0 0 auto;
      color: var(--ok);
      font-size: 12px;
      font-weight: 600;
    }

    .body {
      display: block;
      margin-top: 6px;
      color: var(--muted);
      overflow-wrap: anywhere;
    }

    .empty, .error {
      color: var(--muted);
      padding: 10px;
    }

    .error {
      color: #fca5a5;
    }

    a {
      color: var(--accent);
    }

    @media (max-width: 840px) {
      main {
        width: min(100vw - 20px, 680px);
        padding-top: 18px;
      }

      header, .layout {
        display: block;
      }

      .grid {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }

      .panel {
        margin-top: 12px;
      }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <h1>WebLayer Dashboard</h1>
      <div id="updated" class="meta">Loading...</div>
    </header>

    <section class="grid" aria-label="X stats">
      <a class="stat stat-link" href="/dashboard/posts"><div class="stat-label">Unique posts</div><div id="uniquePosts" class="stat-value">-</div></a>
      <div class="stat"><div class="stat-label">Post encounters</div><div id="postEncounters" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Active feedback</div><div id="activeFeedback" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Active rules</div><div id="activeRules" class="stat-value">-</div></div>
    </section>

    <section class="layout">
      <div>
        <div class="panel">
          <h2>Active Rules</h2>
          <div id="rules" class="list"></div>
        </div>
      </div>

      <div>
        <div class="panel">
          <h2>Recent Feedback</h2>
          <div id="feedback" class="list"></div>
        </div>
        <div class="panel">
          <div class="panel-heading">
            <h2>Pending Rule Proposals</h2>
            <button id="reviewRules" class="action-button" type="button">Review Rule Set</button>
          </div>
          <div id="reviewRulesStatus" class="meta" aria-live="polite"></div>
          <div id="proposals" class="list"></div>
        </div>
      </div>
    </section>
  </main>

  <script>
    const SITE = "x.com";

    async function json(path) {
      const response = await fetch(path, { headers: { "Accept": "application/json" } });
      if (!response.ok) {
        throw new Error(`${path} returned HTTP ${response.status}`);
      }
      return response.json();
    }

    async function postJson(path, body) {
      const response = await fetch(path, {
        method: "POST",
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json"
        },
        body: JSON.stringify(body)
      });
      if (!response.ok) {
        throw new Error(`${path} returned HTTP ${response.status}`);
      }
      return response.json();
    }

    function text(value) {
      return value === null || value === undefined || value === "" ? "-" : String(value);
    }

    function setText(id, value) {
      document.getElementById(id).textContent = text(value);
    }

    function empty(message) {
      const node = document.createElement("div");
      node.className = "empty";
      node.textContent = message;
      return node;
    }

    function errorNode(error) {
      const node = document.createElement("div");
      node.className = "error";
      node.textContent = error instanceof Error ? error.message : String(error);
      return node;
    }

    function renderList(id, items, renderer, emptyMessage) {
      const root = document.getElementById(id);
      root.replaceChildren();
      if (!Array.isArray(items) || items.length === 0) {
        root.append(empty(emptyMessage));
        return;
      }
      for (const item of items) {
        root.append(renderer(item));
      }
    }

    function item(title, body, pill, href) {
      const tagName = href ? "a" : "div";
      const container = document.createElement(tagName);
      const heading = document.createElement("span");
      const titleNode = document.createElement("span");
      const pillNode = document.createElement("span");
      const bodyNode = document.createElement("span");

      container.className = "item";
      if (href) {
        container.href = href;
        container.classList.add("item-link");
      }
      heading.className = "item-title";
      titleNode.textContent = text(title);
      pillNode.className = "pill";
      pillNode.textContent = text(pill);
      bodyNode.className = "body";
      bodyNode.textContent = text(body);
      heading.append(titleNode, pillNode);
      container.append(heading, bodyNode);
      return container;
    }

    function renderRule(rule) {
      const href = `/dashboard/rules/${encodeURIComponent(rule.id)}`;
      return item(rule.title, rule.instruction, `p${rule.priority}`, href);
    }

    function renderRules(rules) {
      const items = Array.isArray(rules.items) ? rules.items : [];
      renderList("rules", items, renderRule, "No active rules.");
    }

    function proposalSummary(proposal) {
      const changes = Array.isArray(proposal.changes) ? proposal.changes.length : 0;
      return `${proposal.source || "proposal"}; ${changes} changes; ${proposal.feedbackCount || 0} feedback rows`;
    }

    function proposalHasActionableChanges(proposal) {
      const changes = Array.isArray(proposal.changes) ? proposal.changes : [];
      return changes.some((change) => change.action !== "noChange");
    }

    async function reviewRuleSet() {
      const button = document.getElementById("reviewRules");
      const status = document.getElementById("reviewRulesStatus");
      button.disabled = true;
      status.textContent = "Reviewing rule set...";

      try {
        const response = await postJson(
          `/v1/rule-proposals?site=${encodeURIComponent(SITE)}`,
          { minFeedback: 1, feedbackLimit: 10 }
        );
        const proposal = response.proposal || {};
        status.textContent = proposalHasActionableChanges(proposal)
          ? `Created ${proposal.id}`
          : "No rule changes proposed.";
        await load();
      } catch (error) {
        status.textContent = error instanceof Error ? error.message : String(error);
      } finally {
        button.disabled = false;
      }
    }

    async function load() {
      try {
        const [stats, feedback, rules, proposals] = await Promise.all([
          json(`/v1/content/stats?site=${encodeURIComponent(SITE)}`),
          json(`/v1/feedback?site=${encodeURIComponent(SITE)}&active=true&limit=10`),
          json(`/v1/rules?site=${encodeURIComponent(SITE)}&status=active&limit=50`),
          json(`/v1/rule-proposals?site=${encodeURIComponent(SITE)}&status=pending&limit=5`)
        ]);
        const pendingProposals = (proposals.items || []).filter(proposalHasActionableChanges);

        setText("uniquePosts", stats.stats && stats.stats.uniqueItems);
        setText("postEncounters", stats.stats && stats.stats.totalEncounters);
        setText("activeFeedback", feedback.totalMatching);
        setText("activeRules", rules.totalMatching);
        renderRules(rules);
        renderList(
          "feedback",
          feedback.items,
          (entry) => item(entry.author || entry.postId || entry.storageKey, entry.reason || entry.text, entry.feedbackKind),
          "No active feedback."
        );
        renderList(
          "proposals",
          pendingProposals,
          (proposal) => item(
            proposal.id,
            proposalSummary(proposal),
            proposal.status,
            `/dashboard/proposals/${encodeURIComponent(proposal.id)}`
          ),
          "No pending rule changes."
        );
        document.getElementById("updated").textContent = `Updated ${new Date().toLocaleTimeString()}`;
      } catch (error) {
        for (const id of ["rules", "feedback", "proposals"]) {
          document.getElementById(id).replaceChildren(errorNode(error));
        }
        document.getElementById("updated").textContent = "Load failed";
      }
    }

    document.getElementById("reviewRules").addEventListener("click", () => {
      void reviewRuleSet();
    });
    void load();
    setInterval(load, 15000);
  </script>
</body>
</html>
"##;

const RULE_DASHBOARD_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>WebLayer Rule</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #0f172a;
      --panel: #111827;
      --panel-2: #172033;
      --border: #334155;
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #7dd3fc;
      --ok: #86efac;
      font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
    }

    main {
      width: min(1180px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 28px 0 40px;
    }

    header {
      display: grid;
      gap: 8px;
      margin-bottom: 18px;
    }

    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: 0;
    }

    h2 {
      margin: 0 0 12px;
      font-size: 15px;
      letter-spacing: 0;
    }

    .meta, .body {
      color: var(--muted);
    }

    .meta {
      font-size: 12px;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-bottom: 12px;
    }

    .layout {
      display: grid;
      grid-template-columns: minmax(0, 1.25fr) minmax(320px, 0.75fr);
      gap: 12px;
    }

    .panel, .stat {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
    }

    .panel {
      padding: 14px;
      min-width: 0;
    }

    .panel + .panel {
      margin-top: 12px;
    }

    .stat {
      padding: 12px;
    }

    .stat-label {
      color: var(--muted);
      font-size: 12px;
    }

    .stat-value {
      margin-top: 4px;
      font-size: 24px;
      font-weight: 700;
    }

    .list {
      display: grid;
      gap: 8px;
    }

    .item {
      padding: 10px;
      border: 1px solid rgba(148, 163, 184, 0.22);
      border-radius: 6px;
      background: var(--panel-2);
    }

    .item-title {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      font-weight: 700;
    }

    .pill {
      flex: 0 0 auto;
      color: var(--ok);
      font-size: 12px;
      font-weight: 600;
    }

    .body {
      margin-top: 6px;
      overflow-wrap: anywhere;
    }

    .detail-row {
      display: grid;
      grid-template-columns: 132px minmax(0, 1fr);
      gap: 10px;
      margin-top: 6px;
    }

    .line {
      color: var(--muted);
      overflow-wrap: anywhere;
    }

    .line-label {
      color: var(--text);
      font-weight: 700;
    }

    .post-text {
      margin-top: 4px;
      padding: 10px;
      border: 1px solid rgba(148, 163, 184, 0.22);
      border-radius: 6px;
      background: rgba(15, 23, 42, 0.42);
      color: var(--text);
      line-height: 1.55;
      overflow-wrap: break-word;
      white-space: pre-wrap;
    }

    .raw-capture {
      margin-top: 8px;
      color: var(--muted);
    }

    .raw-capture summary {
      cursor: pointer;
      color: var(--accent);
    }

    .empty, .error {
      color: var(--muted);
      padding: 10px;
    }

    .error {
      color: #fca5a5;
    }

    a {
      color: var(--accent);
    }

    @media (max-width: 840px) {
      main {
        width: min(100vw - 20px, 680px);
        padding-top: 18px;
      }

      .layout, .grid {
        display: block;
      }

      .stat, .panel {
        margin-top: 12px;
      }

      .detail-row {
        display: block;
      }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <a href="/dashboard">Dashboard</a>
      <h1 id="ruleTitle">Rule</h1>
      <div id="ruleMeta" class="meta">Loading...</div>
    </header>

    <section class="grid" aria-label="Rule stats">
      <div class="stat"><div class="stat-label">Status</div><div id="ruleStatus" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Priority</div><div id="rulePriority" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Caught posts</div><div id="caughtCount" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Source</div><div id="ruleSource" class="stat-value">-</div></div>
    </section>

    <section class="layout">
      <div>
        <div class="panel">
          <h2>Rule Instruction</h2>
          <div id="instruction" class="body">Loading...</div>
        </div>
        <div class="panel">
          <h2>Caught Instances</h2>
          <div id="catches" class="list"></div>
        </div>
      </div>

      <div>
        <div class="panel">
          <h2>Examples</h2>
          <div id="examples" class="list"></div>
        </div>
        <div class="panel">
          <h2>Recent Rule History</h2>
          <div id="audit" class="list"></div>
        </div>
      </div>
    </section>
  </main>

  <script>
    const SITE = "x.com";
    const parts = location.pathname.split("/").filter(Boolean);
    const ruleId = decodeURIComponent(parts[parts.length - 1] || "");

    async function json(path) {
      const response = await fetch(path, { headers: { "Accept": "application/json" } });
      if (!response.ok) {
        throw new Error(`${path} returned HTTP ${response.status}`);
      }
      return response.json();
    }

    function text(value) {
      return value === null || value === undefined || value === "" ? "-" : String(value);
    }

    function setText(id, value) {
      document.getElementById(id).textContent = text(value);
    }

    function formatTime(unixMs) {
      if (!unixMs) {
        return "-";
      }
      return new Date(unixMs).toLocaleString();
    }

    function empty(message) {
      const node = document.createElement("div");
      node.className = "empty";
      node.textContent = message;
      return node;
    }

    function errorNode(error) {
      const node = document.createElement("div");
      node.className = "error";
      node.textContent = error instanceof Error ? error.message : String(error);
      return node;
    }

    function renderList(id, items, renderer, emptyMessage) {
      const root = document.getElementById(id);
      root.replaceChildren();
      if (!Array.isArray(items) || items.length === 0) {
        root.append(empty(emptyMessage));
        return;
      }
      for (const item of items) {
        root.append(renderer(item));
      }
    }

    function item(title, body, pill) {
      const container = document.createElement("div");
      const heading = document.createElement("div");
      const titleNode = document.createElement("div");
      const pillNode = document.createElement("div");
      const bodyNode = document.createElement("div");

      container.className = "item";
      heading.className = "item-title";
      titleNode.textContent = text(title);
      pillNode.className = "pill";
      pillNode.textContent = text(pill);
      bodyNode.className = "body";
      bodyNode.textContent = text(body);
      heading.append(titleNode, pillNode);
      container.append(heading, bodyNode);
      return container;
    }

    function line(label, value, options = {}) {
      const node = document.createElement("div");
      const labelNode = document.createElement("span");
      const valueNode = document.createElement(options.block ? "div" : "span");
      node.className = "detail-row";
      labelNode.className = "line-label";
      labelNode.textContent = `${label}: `;
      valueNode.className = options.block ? "post-text" : "line";
      valueNode.textContent = text(value);
      node.append(labelNode, valueNode);
      return node;
    }

    function rawCaptureDetails(rawText) {
      const details = document.createElement("details");
      const summary = document.createElement("summary");
      const value = document.createElement("div");

      details.className = "raw-capture";
      summary.textContent = "Raw capture";
      value.className = "post-text";
      value.textContent = text(rawText);
      details.append(summary, value);
      return details;
    }

    function catchPill(entry) {
      const confidence = typeof entry.confidence === "number"
        ? `${Math.round(entry.confidence * 100)}%`
        : "";
      return confidence ? `${entry.action} ${confidence}` : entry.action;
    }

    function catchTitle(entry) {
      const content = entry.content || {};
      return content.author || content.contentId || content.storageKey || `event ${entry.eventId}`;
    }

    function readableCapturedText(rawText, author) {
      const raw = String(rawText || "");
      const compact = raw.replace(/\s+/g, " ").trim();
      if (!compact) {
        return { value: "", changed: false };
      }

      let value = compact.replace(/\s*·\s*/g, " · ");
      let strippedChrome = false;
      const authorText = String(author || "").trim().toLowerCase();
      if (authorText) {
        const authorIndex = value.toLowerCase().indexOf(authorText);
        if (authorIndex >= 0 && authorIndex <= 80) {
          value = value.slice(authorIndex + authorText.length).trim();
          strippedChrome = true;
        }
      }
      if (strippedChrome) {
        value = value.replace(
          /^(?:·\s*)?(?:(?:now|\d+[smhd])|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2})\s*/i,
          ""
        ).trim();
      }

      value = value
        .replace(/\s*·\s*/g, " · ")
        .replace(/\s+(?:\d+(?:\.\d+)?[KMB]?){2,}$/i, "")
        .replace(/([^\d\s])(?:\d+(?:\.\d+)?[KMB]?){2,}$/i, "$1")
        .replace(/\s+/g, " ")
        .trim();

      return {
        value: value || compact,
        changed: value.length > 0 && value !== compact
      };
    }

    function catchItem(entry) {
      const content = entry.content || {};
      const capturedText = readableCapturedText(content.text, content.author);
      const container = item(catchTitle(entry), "", catchPill(entry));
      const body = container.querySelector(".body");
      body.replaceChildren(
        line("Why it was caught", entry.reason || "No reason recorded."),
        line("Captured text", capturedText.value || "No stored text.", { block: true }),
        line("Seen", formatTime(entry.caughtAtUnixMs)),
        line("Source", entry.source)
      );
      if (capturedText.changed) {
        body.append(rawCaptureDetails(content.text));
      }
      if (content.url) {
        const link = document.createElement("a");
        link.href = content.url;
        link.target = "_blank";
        link.rel = "noreferrer";
        link.textContent = "Open post";
        const linkLine = document.createElement("div");
        linkLine.className = "line";
        linkLine.append(link);
        body.append(linkLine);
      }
      return container;
    }

    function exampleItems(rule) {
      const examples = rule.examples || {};
      return [
        { title: "Positive", values: examples.positive || [] },
        { title: "Negative", values: examples.negative || [] }
      ];
    }

    async function load() {
      if (!ruleId) {
        throw new Error("Missing rule id in page URL");
      }

      try {
        const [detail, catches] = await Promise.all([
          json(`/v1/rules/${encodeURIComponent(ruleId)}?site=${encodeURIComponent(SITE)}`),
          json(`/v1/rules/${encodeURIComponent(ruleId)}/catches?site=${encodeURIComponent(SITE)}&limit=25`)
        ]);
        const rule = detail.rule || {};

        document.title = `${rule.title || ruleId} - WebLayer Rule`;
        setText("ruleTitle", rule.title || ruleId);
        setText("ruleMeta", rule.id ? `${rule.id}; ${rule.site}` : ruleId);
        setText("ruleStatus", rule.status);
        setText("rulePriority", rule.priority);
        setText("caughtCount", catches.totalMatching);
        setText("ruleSource", rule.createdSource);
        setText("instruction", rule.instruction);
        renderList(
          "catches",
          catches.items,
          catchItem,
          "No caught posts have been recorded for this rule yet."
        );
        renderList(
          "examples",
          exampleItems(rule),
          (entry) => item(entry.title, entry.values.length ? entry.values.join(" | ") : "No examples.", ""),
          "No examples."
        );
        renderList(
          "audit",
          detail.audit,
          (event) => item(event.eventKind, formatTime(event.createdAtUnixMs), event.source),
          "No audit events."
        );
      } catch (error) {
        for (const id of ["catches", "examples", "audit"]) {
          document.getElementById(id).replaceChildren(errorNode(error));
        }
        setText("ruleMeta", "Load failed");
      }
    }

    void load();
  </script>
</body>
</html>
"##;

const PROPOSAL_DASHBOARD_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>WebLayer Rule Proposal</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #0f172a;
      --panel: #111827;
      --panel-2: #172033;
      --border: #334155;
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #7dd3fc;
      --ok: #86efac;
      --warn: #fca5a5;
      font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
    }

    main {
      width: min(1180px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 28px 0 40px;
    }

    header {
      display: grid;
      gap: 8px;
      margin-bottom: 18px;
    }

    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: 0;
    }

    h2 {
      margin: 0 0 12px;
      font-size: 15px;
      letter-spacing: 0;
    }

    button {
      font: inherit;
    }

    .meta, .body {
      color: var(--muted);
    }

    .meta {
      font-size: 12px;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-bottom: 12px;
    }

    .layout {
      display: grid;
      grid-template-columns: minmax(0, 1.25fr) minmax(320px, 0.75fr);
      gap: 12px;
    }

    .panel, .stat {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
    }

    .panel {
      padding: 14px;
      min-width: 0;
    }

    .panel + .panel {
      margin-top: 12px;
    }

    .stat {
      padding: 12px;
    }

    .stat-label {
      color: var(--muted);
      font-size: 12px;
    }

    .stat-value {
      margin-top: 4px;
      font-size: 24px;
      font-weight: 700;
    }

    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 10px;
    }

    .action-button {
      min-height: 32px;
      padding: 0 10px;
      border: 1px solid var(--accent);
      border-radius: 6px;
      background: transparent;
      color: var(--accent);
      cursor: pointer;
      font-weight: 700;
    }

    .reject-button {
      border-color: var(--warn);
      color: var(--warn);
    }

    .action-button:hover,
    .action-button:focus-visible {
      background: rgba(125, 211, 252, 0.12);
      outline: none;
    }

    .reject-button:hover,
    .reject-button:focus-visible {
      background: rgba(252, 165, 165, 0.12);
    }

    .action-button:disabled {
      border-color: var(--border);
      color: var(--muted);
      cursor: default;
    }

    .list {
      display: grid;
      gap: 8px;
    }

    .item {
      padding: 10px;
      border: 1px solid rgba(148, 163, 184, 0.22);
      border-radius: 6px;
      background: var(--panel-2);
    }

    .item-title {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      font-weight: 700;
    }

    .pill {
      flex: 0 0 auto;
      color: var(--ok);
      font-size: 12px;
      font-weight: 600;
    }

    .body {
      margin-top: 8px;
      overflow-wrap: anywhere;
    }

    .detail-row {
      display: grid;
      grid-template-columns: 132px minmax(0, 1fr);
      gap: 10px;
      margin-top: 6px;
    }

    .line {
      color: var(--muted);
      overflow-wrap: anywhere;
      white-space: pre-wrap;
    }

    .line-label {
      color: var(--text);
      font-weight: 700;
    }

    .empty, .error {
      color: var(--muted);
      padding: 10px;
    }

    .error {
      color: var(--warn);
    }

    a {
      color: var(--accent);
    }

    @media (max-width: 840px) {
      main {
        width: min(100vw - 20px, 680px);
        padding-top: 18px;
      }

      .layout, .grid {
        display: block;
      }

      .stat, .panel {
        margin-top: 12px;
      }

      .detail-row {
        display: block;
      }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <a href="/dashboard">Dashboard</a>
      <h1 id="proposalTitle">Rule Proposal</h1>
      <div id="proposalMeta" class="meta">Loading...</div>
    </header>

    <section class="grid" aria-label="Proposal stats">
      <div class="stat"><div class="stat-label">Status</div><div id="proposalStatus" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Feedback rows</div><div id="feedbackCount" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Active rules read</div><div id="activeRuleCount" class="stat-value">-</div></div>
      <div class="stat"><div class="stat-label">Changes</div><div id="changeCount" class="stat-value">-</div></div>
    </section>

    <section class="layout">
      <div>
        <div class="panel">
          <h2>Proposed Changes</h2>
          <div id="changes" class="list"></div>
        </div>
      </div>

      <div>
        <div class="panel">
          <h2>Manual Decision</h2>
          <div class="actions">
            <button id="applyProposal" class="action-button" type="button">Accept Proposal</button>
            <button id="dismissProposal" class="action-button reject-button" type="button">Reject Proposal</button>
          </div>
          <div id="decisionStatus" class="meta" aria-live="polite"></div>
        </div>
        <div class="panel">
          <h2>Changed Rules</h2>
          <div id="changedRules" class="list"></div>
        </div>
      </div>
    </section>
  </main>

  <script>
    const SITE = "x.com";
    const parts = location.pathname.split("/").filter(Boolean);
    const proposalId = decodeURIComponent(parts[parts.length - 1] || "");
    let currentProposal = null;

    async function json(path) {
      const response = await fetch(path, { headers: { "Accept": "application/json" } });
      if (!response.ok) {
        throw new Error(`${path} returned HTTP ${response.status}`);
      }
      return response.json();
    }

    async function postJson(path, body) {
      const response = await fetch(path, {
        method: "POST",
        headers: {
          "Accept": "application/json",
          "Content-Type": "application/json"
        },
        body: JSON.stringify(body)
      });
      if (!response.ok) {
        const fallback = `${path} returned HTTP ${response.status}`;
        try {
          const value = await response.json();
          throw new Error(value.error || fallback);
        } catch (error) {
          if (error instanceof SyntaxError) {
            throw new Error(fallback);
          }
          throw error;
        }
      }
      return response.json();
    }

    function text(value) {
      return value === null || value === undefined || value === "" ? "-" : String(value);
    }

    function setText(id, value) {
      document.getElementById(id).textContent = text(value);
    }

    function formatTime(unixMs) {
      if (!unixMs) {
        return "-";
      }
      return new Date(unixMs).toLocaleString();
    }

    function empty(message) {
      const node = document.createElement("div");
      node.className = "empty";
      node.textContent = message;
      return node;
    }

    function errorNode(error) {
      const node = document.createElement("div");
      node.className = "error";
      node.textContent = error instanceof Error ? error.message : String(error);
      return node;
    }

    function renderList(id, items, renderer, emptyMessage) {
      const root = document.getElementById(id);
      root.replaceChildren();
      if (!Array.isArray(items) || items.length === 0) {
        root.append(empty(emptyMessage));
        return;
      }
      for (const item of items) {
        root.append(renderer(item));
      }
    }

    function line(label, value) {
      const node = document.createElement("div");
      const labelNode = document.createElement("span");
      const valueNode = document.createElement("span");
      node.className = "detail-row";
      labelNode.className = "line-label";
      labelNode.textContent = `${label}: `;
      valueNode.className = "line";
      valueNode.textContent = text(value);
      node.append(labelNode, valueNode);
      return node;
    }

    function item(title, body, pill) {
      const container = document.createElement("div");
      const heading = document.createElement("div");
      const titleNode = document.createElement("div");
      const pillNode = document.createElement("div");
      const bodyNode = document.createElement("div");

      container.className = "item";
      heading.className = "item-title";
      titleNode.textContent = text(title);
      pillNode.className = "pill";
      pillNode.textContent = text(pill);
      bodyNode.className = "body";
      if (body instanceof Node) {
        bodyNode.append(body);
      } else {
        bodyNode.textContent = text(body);
      }
      heading.append(titleNode, pillNode);
      container.append(heading, bodyNode);
      return container;
    }

    function ruleLink(ruleId) {
      const link = document.createElement("a");
      link.href = `/dashboard/rules/${encodeURIComponent(ruleId)}`;
      link.textContent = ruleId;
      return link;
    }

    function examplesText(examples) {
      const value = examples || {};
      const positive = Array.isArray(value.positive) ? value.positive : [];
      const negative = Array.isArray(value.negative) ? value.negative : [];
      const parts = [];
      if (positive.length) {
        parts.push(`positive: ${positive.join(" | ")}`);
      }
      if (negative.length) {
        parts.push(`negative: ${negative.join(" | ")}`);
      }
      return parts.join("; ");
    }

    function proposalHasActionableChanges(proposal) {
      const changes = Array.isArray(proposal.changes) ? proposal.changes : [];
      return changes.some((change) => change.action !== "noChange");
    }

    function changeItem(change, index) {
      const body = document.createElement("div");
      const title = change.title || change.ruleId || `Change ${index + 1}`;
      const ruleId = change.ruleId;

      if (ruleId) {
        const row = line("Rule ID", "");
        const value = row.querySelector(".line");
        value.replaceChildren(ruleLink(ruleId));
        body.append(row);
      }
      body.append(
        line("Status", change.status),
        line("Priority", change.priority),
        line("Instruction", change.instruction),
        line("Rationale", change.rationale),
        line("Evidence", Array.isArray(change.evidenceStorageKeys) ? change.evidenceStorageKeys.join(", ") : ""),
        line("Examples", examplesText(change.examples))
      );

      return item(title, body, change.action || "change");
    }

    function changedRuleItem(rule) {
      const body = document.createElement("div");
      const row = line("Rule ID", "");
      row.querySelector(".line").replaceChildren(ruleLink(rule.id));
      body.append(
        row,
        line("Status", rule.status),
        line("Priority", rule.priority),
        line("Instruction", rule.instruction)
      );
      return item(rule.title || rule.id, body, rule.status);
    }

    function updateDecisionControls(proposal) {
      const actionable = proposal && proposalHasActionableChanges(proposal);
      const pending = actionable && proposal.status === "pending";
      document.getElementById("applyProposal").disabled = !pending;
      document.getElementById("dismissProposal").disabled = !pending;
      document.getElementById("decisionStatus").textContent = actionable
        ? (pending ? "Pending manual decision." : `Proposal is ${proposal ? proposal.status : "unavailable"}.`)
        : "No rule changes proposed; no decision needed.";
    }

    function renderProposal(proposal) {
      currentProposal = proposal;
      const changes = Array.isArray(proposal.changes) ? proposal.changes : [];
      document.title = `${proposal.id || proposalId} - WebLayer Proposal`;
      setText("proposalTitle", proposal.id || proposalId);
      setText(
        "proposalMeta",
        `${proposal.source || "-"}; created ${formatTime(proposal.createdAtUnixMs)}`
      );
      setText("proposalStatus", proposal.status);
      setText("feedbackCount", proposal.feedbackCount);
      setText("activeRuleCount", proposal.activeRuleCount);
      setText("changeCount", changes.length);
      renderList("changes", changes, changeItem, "No proposed changes.");
      updateDecisionControls(proposal);
    }

    async function decide(action) {
      if (!currentProposal || currentProposal.status !== "pending") {
        return;
      }

      const applyButton = document.getElementById("applyProposal");
      const dismissButton = document.getElementById("dismissProposal");
      applyButton.disabled = true;
      dismissButton.disabled = true;
      setText("decisionStatus", action === "apply" ? "Applying proposal..." : "Rejecting proposal...");

      try {
        const response = await postJson(
          `/v1/rule-proposals/${encodeURIComponent(proposalId)}/decision?site=${encodeURIComponent(SITE)}`,
          { action, source: "dashboard" }
        );
        renderProposal(response.proposal || {});
        renderList(
          "changedRules",
          response.changedRules,
          changedRuleItem,
          action === "apply" ? "No rules changed." : "Proposal rejected without changing rules."
        );
        setText("decisionStatus", action === "apply" ? "Proposal accepted." : "Proposal rejected.");
      } catch (error) {
        document.getElementById("changedRules").replaceChildren(errorNode(error));
        updateDecisionControls(currentProposal);
      }
    }

    async function load() {
      if (!proposalId) {
        throw new Error("Missing proposal id in page URL");
      }

      try {
        const detail = await json(
          `/v1/rule-proposals/${encodeURIComponent(proposalId)}?site=${encodeURIComponent(SITE)}`
        );
        renderProposal(detail.proposal || {});
        renderList("changedRules", [], changedRuleItem, "No decision made from this page.");
      } catch (error) {
        document.getElementById("changes").replaceChildren(errorNode(error));
        document.getElementById("changedRules").replaceChildren(errorNode(error));
        setText("proposalMeta", "Load failed");
      }
    }

    document.getElementById("applyProposal").addEventListener("click", () => {
      void decide("apply");
    });
    document.getElementById("dismissProposal").addEventListener("click", () => {
      void decide("dismiss");
    });

    void load();
  </script>
</body>
</html>
"##;

const POSTS_DASHBOARD_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>WebLayer Stored Posts</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: #0f172a;
      --panel: #111827;
      --panel-2: #172033;
      --border: #334155;
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #7dd3fc;
      --ok: #86efac;
      font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
    }

    main {
      width: min(980px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 28px 0 40px;
    }

    header {
      display: grid;
      gap: 8px;
      margin-bottom: 18px;
    }

    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: 0;
    }

    button {
      font: inherit;
    }

    .meta {
      color: var(--muted);
      font-size: 12px;
    }

    .toolbar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 12px;
    }

    .list {
      display: grid;
      gap: 10px;
    }

    .post {
      padding: 12px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
    }

    .post-title {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 10px;
      font-weight: 700;
    }

    .pill {
      flex: 0 0 auto;
      color: var(--ok);
      font-size: 12px;
      font-weight: 600;
    }

    .post-text {
      margin-top: 8px;
      padding: 10px;
      border: 1px solid rgba(148, 163, 184, 0.22);
      border-radius: 6px;
      background: var(--panel-2);
      color: var(--text);
      line-height: 1.55;
      overflow-wrap: break-word;
      white-space: pre-wrap;
    }

    .details {
      display: flex;
      flex-wrap: wrap;
      gap: 8px 14px;
      margin-top: 8px;
      color: var(--muted);
      font-size: 12px;
    }

    .action-button {
      min-height: 32px;
      padding: 0 10px;
      border: 1px solid var(--accent);
      border-radius: 6px;
      background: transparent;
      color: var(--accent);
      cursor: pointer;
      font-weight: 700;
    }

    .action-button:hover,
    .action-button:focus-visible {
      background: rgba(125, 211, 252, 0.12);
      outline: none;
    }

    .action-button:disabled {
      border-color: var(--border);
      color: var(--muted);
      cursor: wait;
    }

    .empty, .error {
      color: var(--muted);
      padding: 10px;
    }

    .error {
      color: #fca5a5;
    }

    a {
      color: var(--accent);
    }

    @media (max-width: 720px) {
      main {
        width: min(100vw - 20px, 680px);
        padding-top: 18px;
      }

      .toolbar, .post-title {
        display: block;
      }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <a href="/dashboard">Dashboard</a>
      <h1>Stored Posts</h1>
      <div id="summary" class="meta">Loading...</div>
    </header>

    <div class="toolbar">
      <div id="status" class="meta" aria-live="polite"></div>
      <button id="loadMore" class="action-button" type="button">Load More</button>
    </div>

    <section id="posts" class="list" aria-label="Stored X posts"></section>
    <div id="sentinel" aria-hidden="true"></div>
  </main>

  <script>
    const SITE = "x.com";
    const LIMIT = 50;
    let offset = 0;
    let totalMatching = null;
    let loading = false;
    let done = false;

    async function json(path) {
      const response = await fetch(path, { headers: { "Accept": "application/json" } });
      if (!response.ok) {
        throw new Error(`${path} returned HTTP ${response.status}`);
      }
      return response.json();
    }

    function text(value) {
      return value === null || value === undefined || value === "" ? "-" : String(value);
    }

    function formatTime(unixMs) {
      if (!unixMs) {
        return "-";
      }
      return new Date(unixMs).toLocaleString();
    }

    function setStatus(message) {
      document.getElementById("status").textContent = message;
    }

    function updateSummary() {
      const total = totalMatching === null ? "-" : totalMatching;
      document.getElementById("summary").textContent = `${offset} of ${total} posts loaded`;
    }

    function empty(message) {
      const node = document.createElement("div");
      node.className = "empty";
      node.textContent = message;
      return node;
    }

    function errorNode(error) {
      const node = document.createElement("div");
      node.className = "error";
      node.textContent = error instanceof Error ? error.message : String(error);
      return node;
    }

    function detail(label, value) {
      const node = document.createElement("span");
      node.textContent = `${label}: ${text(value)}`;
      return node;
    }

    function postNode(post) {
      const container = document.createElement("article");
      const heading = document.createElement("div");
      const title = document.createElement("div");
      const pill = document.createElement("div");
      const body = document.createElement("div");
      const details = document.createElement("div");

      container.className = "post";
      heading.className = "post-title";
      title.textContent = post.author || post.contentId || post.storageKey;
      pill.className = "pill";
      pill.textContent = `${post.seenCount || 0} encounters`;
      body.className = "post-text";
      body.textContent = text(post.text);
      details.className = "details";
      details.append(
        detail("ID", post.contentId),
        detail("First seen", formatTime(post.firstSeenAtUnixMs)),
        detail("Last seen", formatTime(post.lastSeenAtUnixMs))
      );
      if (post.url) {
        const link = document.createElement("a");
        link.href = post.url;
        link.target = "_blank";
        link.rel = "noreferrer";
        link.textContent = "Open post";
        details.append(link);
      }

      heading.append(title, pill);
      container.append(heading, body, details);
      return container;
    }

    async function loadNext() {
      if (loading || done) {
        return;
      }

      loading = true;
      const button = document.getElementById("loadMore");
      button.disabled = true;
      setStatus("Loading posts...");

      try {
        const page = await json(
          `/v1/content?site=${encodeURIComponent(SITE)}&limit=${LIMIT}&offset=${offset}`
        );
        const posts = document.getElementById("posts");
        const items = Array.isArray(page.items) ? page.items : [];
        totalMatching = page.totalMatching;

        if (offset === 0 && items.length === 0) {
          posts.replaceChildren(empty("No stored posts."));
        } else {
          for (const post of items) {
            posts.append(postNode(post));
          }
        }

        offset += items.length;
        done = items.length === 0 || offset >= totalMatching;
        setStatus(done ? "All stored posts loaded." : "Scroll for more posts.");
        updateSummary();
      } catch (error) {
        document.getElementById("posts").append(errorNode(error));
        setStatus("Load failed.");
      } finally {
        loading = false;
        button.disabled = done;
      }
    }

    document.getElementById("loadMore").addEventListener("click", () => {
      void loadNext();
    });

    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        void loadNext();
      }
    }, { rootMargin: "600px" });
    observer.observe(document.getElementById("sentinel"));

    void loadNext();
  </script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dashboard_page_links_existing_x_api_surfaces() {
        let Html(html) = dashboard().await;

        assert!(html.contains("/v1/content/stats?site="));
        assert!(html.contains("/v1/rules?site="));
        assert!(html.contains("/dashboard/posts"));
        assert!(html.contains("/dashboard/rules/"));
        assert!(html.contains("/dashboard/proposals/"));
        assert!(html.contains("/v1/feedback?site="));
        assert!(html.contains("/v1/rule-proposals?site="));
        assert!(html.contains("status=pending"));
        assert!(html.contains("proposalHasActionableChanges"));
        assert!(html.contains("No rule changes proposed."));
        assert!(html.contains("Review Rule Set"));
        assert!(html.contains("method: \"POST\""));
    }

    #[tokio::test]
    async fn rule_dashboard_page_links_rule_detail_surfaces() {
        let Html(html) = rule_dashboard().await;

        assert!(html.contains("/v1/rules/${encodeURIComponent(ruleId)}?site="));
        assert!(html.contains("/catches?site="));
        assert!(html.contains("Caught Instances"));
        assert!(html.contains("readableCapturedText"));
        assert!(html.contains("Captured text"));
        assert!(html.contains("Raw capture"));
    }

    #[tokio::test]
    async fn posts_dashboard_page_links_content_surface() {
        let Html(html) = posts_dashboard().await;

        assert!(html.contains("/v1/content?site="));
        assert!(html.contains("Stored Posts"));
        assert!(html.contains("IntersectionObserver"));
    }

    #[tokio::test]
    async fn proposal_dashboard_page_links_proposal_surfaces() {
        let Html(html) = proposal_dashboard().await;

        assert!(html.contains("/v1/rule-proposals/${encodeURIComponent(proposalId)}?site="));
        assert!(html.contains("/decision?site="));
        assert!(html.contains("Accept Proposal"));
        assert!(html.contains("Reject Proposal"));
        assert!(html.contains("Changed Rules"));
        assert!(html.contains("No rule changes proposed; no decision needed."));
    }
}

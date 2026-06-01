use axum::response::Html;

pub(super) async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

pub(super) async fn rule_dashboard() -> Html<&'static str> {
    Html(RULE_DASHBOARD_HTML)
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
      <div class="stat"><div class="stat-label">Unique posts</div><div id="uniquePosts" class="stat-value">-</div></div>
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
          <h2>Recent Rule Proposals</h2>
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

    async function load() {
      try {
        const [stats, feedback, rules, proposals] = await Promise.all([
          json(`/v1/content/stats?site=${encodeURIComponent(SITE)}`),
          json(`/v1/feedback?site=${encodeURIComponent(SITE)}&active=true&limit=10`),
          json(`/v1/rules?site=${encodeURIComponent(SITE)}&status=active&limit=50`),
          json(`/v1/rule-proposals?site=${encodeURIComponent(SITE)}&limit=5`)
        ]);

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
          proposals.items,
          (proposal) => item(proposal.id, proposalSummary(proposal), proposal.status),
          "No rule proposals."
        );
        document.getElementById("updated").textContent = `Updated ${new Date().toLocaleTimeString()}`;
      } catch (error) {
        for (const id of ["rules", "feedback", "proposals"]) {
          document.getElementById(id).replaceChildren(errorNode(error));
        }
        document.getElementById("updated").textContent = "Load failed";
      }
    }

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

    .line {
      margin-top: 6px;
      color: var(--muted);
      overflow-wrap: anywhere;
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

    function line(label, value) {
      const node = document.createElement("div");
      const labelNode = document.createElement("span");
      node.className = "line";
      labelNode.className = "line-label";
      labelNode.textContent = `${label}: `;
      node.append(labelNode, document.createTextNode(text(value)));
      return node;
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

    function catchItem(entry) {
      const content = entry.content || {};
      const container = item(catchTitle(entry), "", catchPill(entry));
      const body = container.querySelector(".body");
      body.replaceChildren(
        line("Why it was caught", entry.reason || "No reason recorded."),
        line("Post text", content.text || "No stored text."),
        line("Seen", formatTime(entry.caughtAtUnixMs)),
        line("Source", entry.source)
      );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dashboard_page_links_existing_x_api_surfaces() {
        let Html(html) = dashboard().await;

        assert!(html.contains("/v1/content/stats?site="));
        assert!(html.contains("/v1/rules?site="));
        assert!(html.contains("/dashboard/rules/"));
        assert!(html.contains("/v1/feedback?site="));
        assert!(html.contains("/v1/rule-proposals?site="));
    }

    #[tokio::test]
    async fn rule_dashboard_page_links_rule_detail_surfaces() {
        let Html(html) = rule_dashboard().await;

        assert!(html.contains("/v1/rules/${encodeURIComponent(ruleId)}?site="));
        assert!(html.contains("/catches?site="));
        assert!(html.contains("Caught Instances"));
    }
}

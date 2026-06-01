use axum::response::Html;

pub(super) async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
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
      <div class="panel">
        <h2>Active Rules</h2>
        <div id="rules" class="list"></div>
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
        renderList(
          "rules",
          rules.items,
          (rule) => item(rule.title, rule.instruction, `p${rule.priority}`),
          "No active rules."
        );
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
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dashboard_page_links_existing_x_api_surfaces() {
        let Html(html) = dashboard().await;

        assert!(html.contains("/v1/content/stats?site="));
        assert!(html.contains("/v1/rules?site="));
        assert!(html.contains("/v1/feedback?site="));
        assert!(html.contains("/v1/rule-proposals?site="));
    }
}

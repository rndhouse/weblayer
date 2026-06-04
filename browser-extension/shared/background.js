const DEFAULT_DAEMON_ORIGIN = "http://127.0.0.1:17891";
const ANALYZE_PATH = "/v1/dom/analyze";
const FEEDBACK_PATH = "/v1/dom/feedback";
const EXPOSURES_PATH = "/v1/dom/exposures";
const EVENTS_PATH = "/v1/events";
const DEFAULT_DASHBOARD_SITE = "x.com";
const REQUEST_TIMEOUT_MS = 20000;
const MAX_ELEMENTS_PER_REQUEST = 16;
const REQUEST_GC_MS = 60000;
const PENDING_RESPONSE_TIMEOUT_MS = 1000;

let socket = null;
let socketOrigin = null;
let socketPromise = null;
let nextRequestId = 1;

const pendingRequests = new Map();

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message !== "object") {
    return false;
  }

  if (message.type === "weblayer:analyzeDom") {
    analyzeDom(message, sender)
      .then((commands) => sendResponse({ ok: true, commands }))
      .catch((error) => {
        sendResponse({
          ok: false,
          error: error instanceof Error ? error.message : String(error)
        });
      });

    return true;
  }

  if (message.type === "weblayer:feedback") {
    sendFeedback(message)
      .then((commands) => sendResponse({ ok: true, commands }))
      .catch((error) => {
        sendResponse({
          ok: false,
          error: error instanceof Error ? error.message : String(error)
        });
      });

    return true;
  }

  if (message.type === "weblayer:viewportExposures") {
    sendViewportExposures(message)
      .then((result) => sendResponse({ ok: true, ...result }))
      .catch((error) => {
        sendResponse({
          ok: false,
          error: error instanceof Error ? error.message : String(error)
        });
      });

    return true;
  }

  return false;
});

async function analyzeDom(message, sender) {
  const elements = Array.isArray(message.elements)
    ? message.elements.slice(0, MAX_ELEMENTS_PER_REQUEST).map(normalizeElement)
    : [];
  if (elements.length === 0) {
    return [];
  }

  const settings = await getSettings();
  const page = normalizePage(message.page);
  const tabId = sender && sender.tab && Number.isInteger(sender.tab.id) ? sender.tab.id : null;

  if (tabId !== null) {
    try {
      return await sendAnalyzeDomOverSocket(settings.daemonOrigin, tabId, { page, elements });
    } catch (_error) {
      // Fall through to REST so a disconnected WebSocket does not block development.
    }
  }

  return analyzeDomOverRest(settings.daemonOrigin, { page, elements });
}

async function sendAnalyzeDomOverSocket(origin, tabId, body) {
  const ws = await ensureSocket(origin);
  const requestId = nextRequestIdString();
  const timeoutId = setTimeout(() => {
    finishPendingRequest(requestId);
  }, REQUEST_GC_MS);

  return new Promise((resolve) => {
    const pendingTimeoutId = setTimeout(() => {
      const pendingRequest = pendingRequests.get(requestId);
      if (pendingRequest && pendingRequest.resolvePending) {
        pendingRequest.resolvePending = null;
        pendingRequest.pendingTimeoutId = null;
        resolve([]);
      }
    }, PENDING_RESPONSE_TIMEOUT_MS);

    pendingRequests.set(requestId, {
      daemonOrigin: normalizeOrigin(origin),
      tabId,
      timeoutId,
      pendingTimeoutId,
      resolvePending: resolve
    });

    ws.send(
      JSON.stringify({
        type: "analyzeDom",
        requestId,
        page: body.page,
        elements: body.elements
      })
    );
  });
}

async function analyzeDomOverRest(origin, body) {
  const response = await postJson(origin, ANALYZE_PATH, body);

  if (!Array.isArray(response.commands)) {
    throw new Error("Daemon response must include a commands array.");
  }

  return response.commands.map((command) => normalizeCommand(command, origin));
}

async function sendFeedback(message) {
  const settings = await getSettings();
  const feedback = normalizeFeedback(message.feedback);
  const reason = stringOrEmpty(message.reason);
  const page = normalizePage(message.page);
  const element = normalizeElement(message.element);
  const body = {
    feedback,
    reason,
    page,
    element,
    feedbackContextId: normalizeFeedbackContextId(message.feedbackContextId)
  };

  const response = await postJson(settings.daemonOrigin, FEEDBACK_PATH, body);

  if (!Array.isArray(response.commands)) {
    throw new Error("Daemon response must include a commands array.");
  }

  return response.commands.map((command) => normalizeCommand(command, settings.daemonOrigin));
}

async function sendViewportExposures(message) {
  const exposures = Array.isArray(message.exposures)
    ? message.exposures.map(normalizeViewportExposure).filter((exposure) => (
      exposure.element.clientId && exposure.visibleDurationMs > 0
    ))
    : [];
  if (exposures.length === 0) {
    return { storedCount: 0 };
  }

  const settings = await getSettings();
  const page = normalizePage(message.page);
  const response = await postJson(settings.daemonOrigin, EXPOSURES_PATH, { page, exposures });

  return {
    storedCount: Number.isFinite(response.storedCount) ? response.storedCount : 0
  };
}

function ensureSocket(origin) {
  const wsOrigin = websocketOrigin(origin);
  if (socket && socket.readyState === WebSocket.OPEN && socketOrigin === wsOrigin) {
    return Promise.resolve(socket);
  }

  if (socketPromise && socketOrigin === wsOrigin) {
    return socketPromise;
  }

  closeSocket();
  socketOrigin = wsOrigin;
  socketPromise = new Promise((resolve, reject) => {
    const ws = new WebSocket(`${wsOrigin}${EVENTS_PATH}`);
    let settled = false;

    ws.addEventListener("open", () => {
      socket = ws;
      settled = true;
      socketPromise = null;
      resolve(ws);
    });

    ws.addEventListener("message", (event) => {
      handleSocketMessage(event.data);
    });

    ws.addEventListener("close", () => {
      if (socket === ws) {
        socket = null;
      }
      if (!settled) {
        socketPromise = null;
        reject(new Error("Daemon WebSocket closed before opening."));
      }
    });

    ws.addEventListener("error", () => {
      if (!settled) {
        socketPromise = null;
        reject(new Error("Daemon WebSocket failed to open."));
      }
    });
  });

  return socketPromise;
}

function closeSocket() {
  if (socket) {
    socket.close();
    socket = null;
  }
  socketPromise = null;
}

function handleSocketMessage(data) {
  let event;
  try {
    event = JSON.parse(String(data || ""));
  } catch (_error) {
    return;
  }

  if (!event || event.type !== "commands") {
    return;
  }

  const requestId = stringOrEmpty(event.requestId);
  const pendingRequest = pendingRequests.get(requestId);
  if (!pendingRequest) {
    return;
  }

  const commands = Array.isArray(event.commands)
    ? event.commands.map((command) => normalizeCommand(command, pendingRequest.daemonOrigin))
    : [];
  let shouldPush = true;

  if ((event.phase === "pending" || event.phase === "final") && pendingRequest.resolvePending) {
    if (pendingRequest.pendingTimeoutId) {
      clearTimeout(pendingRequest.pendingTimeoutId);
    }
    pendingRequest.pendingTimeoutId = null;
    const resolvePending = pendingRequest.resolvePending;
    pendingRequest.resolvePending = null;
    shouldPush = false;
    resolvePending(commands);

    if (event.phase === "final") {
      finishPendingRequest(requestId);
    }
  }

  if (commands.length === 0) {
    if (event.phase === "final") {
      finishPendingRequest(requestId);
    }
    return;
  }

  if (!shouldPush) {
    return;
  }

  chrome.tabs.sendMessage(
    pendingRequest.tabId,
    {
      type: "weblayer:applyCommands",
      commands
    },
    () => {
      chrome.runtime.lastError;
    }
  );

  if (event.phase === "final") {
    finishPendingRequest(requestId);
  }
}

function finishPendingRequest(requestId) {
  const pendingRequest = pendingRequests.get(requestId);
  if (!pendingRequest) {
    return;
  }

  clearTimeout(pendingRequest.timeoutId);
  if (pendingRequest.pendingTimeoutId) {
    clearTimeout(pendingRequest.pendingTimeoutId);
  }
  if (pendingRequest.resolvePending) {
    pendingRequest.resolvePending([]);
  }
  pendingRequests.delete(requestId);
}

function normalizePage(page) {
  return {
    url: stringOrEmpty(page && page.url),
    title: stringOrNull(page && page.title),
    capturedAt: stringOrNull(page && page.capturedAt)
  };
}

function normalizeElement(element) {
  return {
    clientId: stringOrEmpty(element.clientId),
    selector: stringOrNull(element.selector),
    tagName: stringOrNull(element.tagName),
    role: stringOrNull(element.role),
    text: stringOrEmpty(element.text),
    html: stringOrNull(element.html),
    attributes: Array.isArray(element.attributes)
      ? element.attributes.map(normalizeAttribute)
      : [],
    links: Array.isArray(element.links) ? element.links.map(normalizeLink) : [],
    snapshotHash: stringOrNull(element.snapshotHash),
    capturedAt: stringOrNull(element.capturedAt),
    metadata: objectOrNull(element.metadata)
  };
}

function normalizeAttribute(attribute) {
  return {
    name: stringOrEmpty(attribute && attribute.name),
    value: stringOrEmpty(attribute && attribute.value)
  };
}

function normalizeLink(link) {
  return {
    href: stringOrEmpty(link && link.href),
    text: stringOrNull(link && link.text),
    ariaLabel: stringOrNull(link && link.ariaLabel)
  };
}

function normalizeViewportExposure(exposure) {
  return {
    element: normalizeElement(exposure && exposure.element),
    firstVisibleAt: stringOrNull(exposure && exposure.firstVisibleAt),
    lastVisibleAt: stringOrNull(exposure && exposure.lastVisibleAt),
    visibleDurationMs: numberOrZero(exposure && exposure.visibleDurationMs),
    maxVisibleRatio: numberOrZero(exposure && exposure.maxVisibleRatio),
    viewportWidth: integerOrNull(exposure && exposure.viewportWidth),
    viewportHeight: integerOrNull(exposure && exposure.viewportHeight)
  };
}

function normalizeCommand(command, daemonOrigin = DEFAULT_DAEMON_ORIGIN) {
  const action = stringOrEmpty(command.action);
  const allowedActions = new Set([
    "keep",
    "hide",
    "dim",
    "insertLabel",
    "insertFeedbackControl",
    "replaceText",
    "showDebugStats"
  ]);
  const target = command.target && typeof command.target === "object" ? command.target : {};
  const normalizedAction = allowedActions.has(action) ? action : "keep";

  return {
    action: normalizedAction,
    target: {
      clientId: stringOrEmpty(target.clientId),
      selector: stringOrNull(target.selector),
      mustMatchSnapshotHash: stringOrNull(target.mustMatchSnapshotHash)
    },
    label: stringOrNull(command.label),
    text: stringOrNull(command.text),
    reason: stringOrNull(command.reason),
    confidence: Number.isFinite(command.confidence) ? command.confidence : null,
    feedbackContextId: normalizedAction === "insertFeedbackControl"
      ? normalizeFeedbackContextId(command.feedbackContextId)
      : stringOrNull(command.feedbackContextId),
    matchedRuleIds: Array.isArray(command.matchedRuleIds)
      ? command.matchedRuleIds.map(stringOrEmpty).filter(Boolean)
      : [],
    debugStats: normalizedAction === "showDebugStats"
      ? normalizeDebugStats(command.debugStats, daemonOrigin)
      : null
  };
}

function normalizeDebugStats(stats, daemonOrigin) {
  if (!stats || typeof stats !== "object") {
    return null;
  }

  const site = stringOrEmpty(stats.site) || DEFAULT_DASHBOARD_SITE;
  return {
    site: site,
    title: stringOrEmpty(stats.title) || "WebLayer stats",
    dashboardUrl: safeDashboardUrl(stats.dashboardUrl, daemonOrigin, site),
    generatedAtUnixMs: Number.isFinite(stats.generatedAtUnixMs)
      ? stats.generatedAtUnixMs
      : null,
    sections: Array.isArray(stats.sections)
      ? stats.sections.map(normalizeDebugStatsSection).filter((section) => (
        section.title || section.metrics.length > 0
      ))
      : []
  };
}

function safeDashboardUrl(value, daemonOrigin, site) {
  const normalizedOrigin = normalizeOrigin(daemonOrigin);
  const candidate = stringOrNull(value);

  if (candidate) {
    try {
      const url = new URL(candidate);
      if (url.origin === normalizedOrigin) {
        return url.href;
      }
    } catch (_error) {
      // Fall back to the configured local daemon origin.
    }
  }

  return dashboardUrl(normalizedOrigin, site);
}

function dashboardUrl(origin, site) {
  return `${normalizeOrigin(origin)}${dashboardPath(site)}`;
}

function dashboardPath(site) {
  const normalizedSite = stringOrEmpty(site) || DEFAULT_DASHBOARD_SITE;
  return `/${encodeURIComponent(normalizedSite)}/dashboard`;
}

function normalizeDebugStatsSection(section) {
  return {
    title: stringOrEmpty(section && section.title),
    metrics: Array.isArray(section && section.metrics)
      ? section.metrics.map(normalizeDebugStatsMetric).filter((metric) => (
        metric.label || metric.value || metric.detail
      ))
      : []
  };
}

function normalizeDebugStatsMetric(metric) {
  return {
    label: stringOrEmpty(metric && metric.label),
    value: stringOrEmpty(metric && metric.value),
    detail: stringOrNull(metric && metric.detail)
  };
}

function normalizeFeedbackContextId(value) {
  const id = stringOrEmpty(value);
  if (!id) {
    throw new Error("Feedback context ID is required.");
  }
  return id;
}

function normalizeFeedback(value) {
  const feedback = stringOrEmpty(value);
  const allowedFeedback = new Set(["thumbsDown", "undoThumbsDown", "updateReason"]);
  return allowedFeedback.has(feedback) ? feedback : "thumbsDown";
}

async function getSettings() {
  return new Promise((resolve) => {
    chrome.storage.local.get(
      { daemonOrigin: DEFAULT_DAEMON_ORIGIN },
      (settings) => {
        resolve({
          daemonOrigin: normalizeOrigin(settings.daemonOrigin)
        });
      }
    );
  });
}

async function postJson(origin, path, body) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  try {
    const response = await fetch(`${origin}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify(body),
      signal: controller.signal
    });

    if (!response.ok) {
      throw new Error(`Daemon returned HTTP ${response.status}.`);
    }

    return await response.json();
  } finally {
    clearTimeout(timeout);
  }
}

function normalizeOrigin(value) {
  const origin = stringOrEmpty(value).trim().replace(/\/+$/, "");

  try {
    const url = new URL(origin || DEFAULT_DAEMON_ORIGIN);
    if (url.protocol !== "http:") {
      return DEFAULT_DAEMON_ORIGIN;
    }
    if (url.hostname !== "127.0.0.1" && url.hostname !== "localhost") {
      return DEFAULT_DAEMON_ORIGIN;
    }
    return url.origin;
  } catch (_error) {
    return DEFAULT_DAEMON_ORIGIN;
  }
}

function websocketOrigin(origin) {
  const url = new URL(normalizeOrigin(origin));
  url.protocol = "ws:";
  return url.origin;
}

function nextRequestIdString() {
  const requestId = `dom:${Date.now()}:${nextRequestId}`;
  nextRequestId += 1;
  return requestId;
}

function stringOrEmpty(value) {
  return typeof value === "string" ? value : "";
}

function stringOrNull(value) {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function numberOrZero(value) {
  return Number.isFinite(value) && value > 0 ? value : 0;
}

function integerOrNull(value) {
  return Number.isInteger(value) ? value : null;
}

function objectOrNull(value) {
  return value && typeof value === "object" && !Array.isArray(value) ? value : null;
}

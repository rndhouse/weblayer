const SCAN_DEBOUNCE_MS = 250;
const MAX_REGIONS_PER_SCAN = 16;
const MAX_TEXT_CHARS = 20000;
const MAX_HTML_CHARS = 60000;
const MAX_LINKS = 80;
const MAX_ATTRIBUTES = 40;
const FEEDBACK_REASON_SAVE_DEBOUNCE_MS = 550;
const DEBUG_STATS_PANEL_ID = "weblayer-debug-stats-panel";
const VIEWPORT_EXPOSURE_MIN_VISIBLE_RATIO = 0.5;
const VIEWPORT_EXPOSURE_MIN_VISIBLE_MS = 750;
const VIEWPORT_EXPOSURE_FLUSH_MS = 500;
const MAX_VIEWPORT_EXPOSURES_PER_REQUEST = 50;
const FEEDBACK_REASON_PRESETS = [
  "Low information",
  "Rage bait",
  "Spam",
  "AI slop",
  "Not interested"
];

let nextGeneratedId = 1;
let nextFeedbackSaveId = 1;
let scanTimer = null;
let requestInFlight = false;
let activeCaptureContextKey = null;
let nextHiddenId = 1;
let viewportExposureObserver = null;
let viewportExposureStates = new WeakMap();
let viewportExposureFlushTimer = null;
let viewportExposureInFlight = false;

const elementIds = new WeakMap();
const elementsByClientId = new Map();
const snapshotsByClientId = new Map();
const feedbackReasonTimers = new WeakMap();
const queuedSnapshots = [];
const queuedViewportExposures = [];

scheduleScan();

const observer = new MutationObserver(() => {
  scheduleScan();
});

observer.observe(document.documentElement, {
  childList: true,
  subtree: true,
  characterData: true
});

document.addEventListener("click", handleWebLayerClick, true);
document.addEventListener("pointerdown", handleWebLayerPointerDown, true);
document.addEventListener("visibilitychange", handleVisibilityChange, true);
if (typeof window.addEventListener === "function") {
  window.addEventListener("pagehide", handlePageHide, true);
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message || message.type !== "weblayer:applyCommands") {
    return false;
  }

  applyCommands(Array.isArray(message.commands) ? message.commands : []);
  sendResponse({ ok: true });
  return false;
});

function scheduleScan() {
  if (scanTimer !== null) {
    return;
  }

  scanTimer = setTimeout(() => {
    scanTimer = null;
    scanForRegions();
  }, SCAN_DEBOUNCE_MS);
}

function scanForRegions() {
  const context = refreshCaptureContext();
  if (!context) {
    return;
  }

  for (const element of collectRegions(context)) {
    const snapshot = snapshotElement(element, context);
    if (!snapshot) {
      continue;
    }

    if (
      element.dataset.weblayerKeepVisibleAfterFeedbackHash &&
      element.dataset.weblayerKeepVisibleAfterFeedbackHash !== snapshot.snapshotHash
    ) {
      delete element.dataset.weblayerKeepVisibleAfterFeedbackHash;
    }

    const alreadyCaptured = element.dataset.weblayerSnapshotHash === snapshot.snapshotHash;
    element.dataset.weblayerSnapshotHash = snapshot.snapshotHash;
    setSnapshotCaptureContextKey(snapshot, context.key);
    elementsByClientId.set(snapshot.clientId, element);
    snapshotsByClientId.set(snapshot.clientId, snapshot);
    trackViewportExposure(element, snapshot, context);
    if (alreadyCaptured) {
      continue;
    }

    element.dataset.weblayerState = "queued";
    installOptimisticFeedbackControl(element, snapshot);
    queuedSnapshots.push(snapshot);
  }

  flushQueue(context);
}

function collectRegions(context) {
  const candidates = context.collectCandidates()
    .filter((candidate) => context.isSupportedElement(candidate))
    .filter(isVisibleRegion)
    .filter(hasSnapshotContent);
  const selected = [];

  for (const candidate of candidates) {
    if (selected.some((element) => element.contains(candidate) || candidate.contains(element))) {
      continue;
    }

    selected.push(candidate);
    if (selected.length >= MAX_REGIONS_PER_SCAN) {
      break;
    }
  }

  return selected.sort((left, right) => {
    const position = left.compareDocumentPosition(right);
    return position & Node.DOCUMENT_POSITION_PRECEDING ? 1 : -1;
  });
}

function refreshCaptureContext() {
  const context = currentCaptureContext();
  const nextKey = context ? context.key : null;
  if (nextKey !== activeCaptureContextKey) {
    clearCaptureState();
    activeCaptureContextKey = nextKey;
  }

  return context;
}

function currentCaptureContext() {
  const adapters = window.WebLayerSiteAdapters;
  if (!adapters || typeof adapters.current !== "function") {
    return null;
  }

  return adapters.current(window.location, document);
}

function clearCaptureState() {
  finalizeVisibleViewportExposures();
  if (viewportExposureObserver) {
    viewportExposureObserver.disconnect();
    viewportExposureObserver = null;
  }
  viewportExposureStates = new WeakMap();
  queuedSnapshots.length = 0;
  elementsByClientId.clear();
  snapshotsByClientId.clear();
}

function isVisibleRegion(element) {
  if (!(element instanceof Element)) {
    return false;
  }

  const rect = element.getBoundingClientRect();
  if (rect.width < 1 || rect.height < 1) {
    return false;
  }
  if (rect.bottom < 0 || rect.top > window.innerHeight * 1.5) {
    return false;
  }

  const style = getComputedStyle(element);
  return style.display !== "none" && style.visibility !== "hidden" && style.opacity !== "0";
}

function hasSnapshotContent(element) {
  const clone = cloneForSnapshot(element);
  const text = normalizeText(clone.innerText || clone.textContent || "");
  return text.length > 0 || clone.querySelector("a[href]") !== null;
}

function snapshotElement(element, context = refreshCaptureContext()) {
  if (!context || !context.isSupportedElement(element)) {
    return null;
  }

  const clone = cloneForSnapshot(element);
  const text = truncate(normalizeText(clone.innerText || clone.textContent || ""), MAX_TEXT_CHARS);
  const links = snapshotLinks(clone);

  if (!text && links.length === 0) {
    return null;
  }

  const clientId = getClientId(element);
  const html = truncate(clone.outerHTML || "", MAX_HTML_CHARS);
  const attributes = snapshotAttributes(element);
  const selector = cssPath(element);
  const metadata = snapshotMetadata(context, element, { text, links });
  const snapshotHash = stableHash(
    JSON.stringify({
      url: location.href,
      selector,
      text,
      links: links.map((link) => link.href)
    })
  );

  return {
    clientId,
    selector,
    tagName: element.tagName.toLowerCase(),
    role: element.getAttribute("role"),
    text,
    html,
    attributes,
    links,
    snapshotHash,
    capturedAt: new Date().toISOString(),
    metadata
  };
}

function snapshotMetadata(context, element, snapshot) {
  if (!context || typeof context.metadataForElement !== "function") {
    return null;
  }

  const metadata = context.metadataForElement(element, snapshot);
  return metadata && typeof metadata === "object" ? metadata : null;
}

function setSnapshotCaptureContextKey(snapshot, key) {
  Object.defineProperty(snapshot, "captureContextKey", {
    value: key,
    enumerable: false
  });
}

function snapshotForMessage(snapshot) {
  return { ...snapshot };
}

function cloneForSnapshot(element) {
  const clone = element.cloneNode(true);
  for (const weblayerElement of clone.querySelectorAll(".weblayer-badge")) {
    weblayerElement.remove();
  }
  for (const weblayerElement of clone.querySelectorAll("[data-weblayer-ui='true']")) {
    weblayerElement.remove();
  }
  return clone;
}

function snapshotLinks(root) {
  return Array.from(root.querySelectorAll("a[href]"))
    .slice(0, MAX_LINKS)
    .map((anchor) => ({
      href: absoluteUrl(anchor.getAttribute("href")),
      text: stringOrNull(normalizeText(anchor.innerText || anchor.textContent || "")),
      ariaLabel: stringOrNull(anchor.getAttribute("aria-label"))
    }))
    .filter((link) => link.href.length > 0);
}

function snapshotAttributes(element) {
  return Array.from(element.attributes)
    .filter((attribute) => !attribute.name.startsWith("data-weblayer"))
    .slice(0, MAX_ATTRIBUTES)
    .map((attribute) => ({
      name: attribute.name,
      value: truncate(attribute.value, 1000)
    }));
}

function getClientId(element) {
  const existingId = elementIds.get(element);
  if (existingId) {
    return existingId;
  }

  const id = `dom:${nextGeneratedId}`;
  nextGeneratedId += 1;
  elementIds.set(element, id);
  return id;
}

function pageSnapshot() {
  return {
    url: location.href,
    title: document.title || null,
    capturedAt: new Date().toISOString()
  };
}

async function flushQueue(context = refreshCaptureContext()) {
  if (!context) {
    queuedSnapshots.length = 0;
    return;
  }

  if (requestInFlight || queuedSnapshots.length === 0) {
    return;
  }

  requestInFlight = true;
  const batch = queuedSnapshots
    .splice(0, MAX_REGIONS_PER_SCAN)
    .filter((snapshot) => snapshot.captureContextKey === context.key);

  if (batch.length === 0) {
    requestInFlight = false;
    if (queuedSnapshots.length > 0) {
      setTimeout(flushQueue, 100);
    }
    return;
  }

  try {
    for (const snapshot of batch) {
      const element = elementsByClientId.get(snapshot.clientId);
      if (element) {
        element.dataset.weblayerState = "pending";
      }
    }

    const response = await sendMessage({
      type: "weblayer:analyzeDom",
      page: pageSnapshot(),
      elements: batch.map(snapshotForMessage)
    });

    if (!response || !response.ok) {
      throw new Error(response && response.error ? response.error : "Daemon request failed.");
    }

    applyCommands(response.commands || []);
  } catch (error) {
    markBatchUnavailable(batch, error);
  } finally {
    requestInFlight = false;
    if (queuedSnapshots.length > 0) {
      setTimeout(flushQueue, 100);
    }
  }
}

function trackViewportExposure(element, snapshot, context) {
  if (!context || context.id !== "x.com" || !("IntersectionObserver" in window)) {
    return;
  }

  let state = viewportExposureStates.get(element);
  if (state) {
    state.snapshot = snapshot;
    state.contextKey = context.key;
    return;
  }

  state = {
    snapshot,
    contextKey: context.key,
    visibleSinceMs: null,
    firstVisibleAt: null,
    maxVisibleRatio: 0
  };
  viewportExposureStates.set(element, state);
  viewportExposureObserverForPage().observe(element);
}

function viewportExposureObserverForPage() {
  if (viewportExposureObserver) {
    return viewportExposureObserver;
  }

  viewportExposureObserver = new IntersectionObserver(handleViewportIntersections, {
    threshold: [0, VIEWPORT_EXPOSURE_MIN_VISIBLE_RATIO, 0.75, 1]
  });
  return viewportExposureObserver;
}

function handleViewportIntersections(entries) {
  const nowMs = performanceNow();

  for (const entry of entries) {
    const state = viewportExposureStates.get(entry.target);
    if (!state) {
      continue;
    }

    const visibleRatio = Number.isFinite(entry.intersectionRatio)
      ? entry.intersectionRatio
      : 0;
    if (visibleRatio >= VIEWPORT_EXPOSURE_MIN_VISIBLE_RATIO && isVisibleRegion(entry.target)) {
      if (state.visibleSinceMs === null) {
        state.visibleSinceMs = nowMs;
        state.firstVisibleAt = new Date().toISOString();
        state.maxVisibleRatio = visibleRatio;
      } else {
        state.maxVisibleRatio = Math.max(state.maxVisibleRatio, visibleRatio);
      }
      continue;
    }

    finalizeViewportExposureState(state, nowMs);
  }
}

function handleVisibilityChange() {
  if (document.visibilityState === "hidden") {
    finalizeAndFlushViewportExposures();
  }
}

function handlePageHide() {
  finalizeAndFlushViewportExposures();
}

function finalizeAndFlushViewportExposures() {
  finalizeVisibleViewportExposures();
  flushViewportExposureQueue();
}

function finalizeVisibleViewportExposures() {
  const nowMs = performanceNow();
  for (const element of Array.from(document.querySelectorAll("[data-weblayer-snapshot-hash]"))) {
    const state = viewportExposureStates.get(element);
    if (state) {
      finalizeViewportExposureState(state, nowMs);
    }
  }
}

function finalizeViewportExposureState(state, nowMs) {
  if (state.visibleSinceMs === null) {
    return;
  }

  const visibleDurationMs = Math.max(0, Math.round(nowMs - state.visibleSinceMs));
  if (visibleDurationMs >= VIEWPORT_EXPOSURE_MIN_VISIBLE_MS) {
    queuedViewportExposures.push({
      element: snapshotForMessage(state.snapshot),
      firstVisibleAt: state.firstVisibleAt,
      lastVisibleAt: new Date().toISOString(),
      visibleDurationMs,
      maxVisibleRatio: state.maxVisibleRatio,
      viewportWidth: Number.isFinite(window.innerWidth) ? Math.round(window.innerWidth) : null,
      viewportHeight: Number.isFinite(window.innerHeight) ? Math.round(window.innerHeight) : null
    });
    scheduleViewportExposureFlush();
  }

  state.visibleSinceMs = null;
  state.firstVisibleAt = null;
  state.maxVisibleRatio = 0;
}

function scheduleViewportExposureFlush() {
  if (viewportExposureFlushTimer !== null) {
    return;
  }

  viewportExposureFlushTimer = setTimeout(() => {
    viewportExposureFlushTimer = null;
    flushViewportExposureQueue();
  }, VIEWPORT_EXPOSURE_FLUSH_MS);
}

async function flushViewportExposureQueue() {
  if (viewportExposureInFlight || queuedViewportExposures.length === 0) {
    return;
  }

  viewportExposureInFlight = true;
  const batch = queuedViewportExposures.splice(0, MAX_VIEWPORT_EXPOSURES_PER_REQUEST);

  try {
    await sendMessage({
      type: "weblayer:viewportExposures",
      page: pageSnapshot(),
      exposures: batch
    });
  } catch (_error) {
    // Exposure data is best-effort; avoid building an unbounded queue when the daemon is down.
  } finally {
    viewportExposureInFlight = false;
    if (queuedViewportExposures.length > 0) {
      scheduleViewportExposureFlush();
    }
  }
}

function performanceNow() {
  return window.performance && typeof window.performance.now === "function"
    ? window.performance.now()
    : Date.now();
}

function sendMessage(message) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(message, (response) => {
      const error = chrome.runtime.lastError;
      if (error) {
        reject(new Error(error.message));
        return;
      }

      resolve(response);
    });
  });
}

function applyCommands(commands) {
  for (const command of commands) {
    if (command.action === "showDebugStats") {
      renderDebugStatsPanel(command.debugStats);
      continue;
    }

    const element = resolveTarget(command.target);
    if (!element || !targetStillMatches(element, command.target)) {
      continue;
    }

    if (command.action === "insertFeedbackControl") {
      insertFeedbackControl(element, command);
      continue;
    }

    if (command.action === "hide" && shouldKeepVisibleAfterFeedback(element, command)) {
      element.dataset.weblayerState = "feedbackActive";
      continue;
    }

    if (command.action === "hide") {
      const hiddenElement = hideContainerForElement(element);
      const wasHiddenExpanded = hiddenElement.classList.contains("weblayer-hidden--expanded");
      clearWebLayerChanges(hiddenElement);
      hiddenElement.dataset.weblayerState = command.action || "hide";
      collapseHiddenElement(hiddenElement, command, { expanded: wasHiddenExpanded });
      continue;
    }

    clearWebLayerChanges(element);
    element.dataset.weblayerState = command.action || "keep";

    if (command.action === "keep") {
      continue;
    }

    if (command.action === "dim") {
      element.classList.add("weblayer-dimmed");
      insertBadge(element, command);
      continue;
    }

    if (command.action === "insertLabel") {
      insertBadge(element, command);
      continue;
    }

    if (command.action === "replaceText" && command.text) {
      replaceRegionText(element, command.text);
      insertBadge(element, command);
    }
  }
}

function renderDebugStatsPanel(stats) {
  if (!stats || !Array.isArray(stats.sections)) {
    removeDebugStatsPanel();
    return;
  }

  const mount = debugStatsMountTarget();
  let panel = document.getElementById(DEBUG_STATS_PANEL_ID);
  if (!panel) {
    panel = document.createElement("div");
    panel.id = DEBUG_STATS_PANEL_ID;
    panel.className = "weblayer-debug-stats-panel";
    panel.dataset.weblayerUi = "true";
    panel.setAttribute("aria-live", "polite");
  }
  panel.classList.toggle("weblayer-debug-stats-panel--inline", mount !== null);
  panel.classList.toggle("weblayer-debug-stats-panel--overlay", mount === null);

  panel.replaceChildren(createDebugStatsHeader(stats));
  for (const section of stats.sections) {
    panel.append(createDebugStatsSection(section));
  }

  if (mount && placeDebugStatsPanel(panel, mount)) {
    return;
  }

  const pageRoot = document.body || document.documentElement;
  if (panel.parentElement !== pageRoot) {
    pageRoot.append(panel);
  }
}

function removeDebugStatsPanel() {
  const panel = document.getElementById(DEBUG_STATS_PANEL_ID);
  if (panel) {
    panel.remove();
  }
}

function placeDebugStatsPanel(panel, mount) {
  const element = mount.element;
  if (!(element instanceof Element) || !document.documentElement.contains(element)) {
    return false;
  }

  if (mount.placement === "before" && element.parentElement) {
    if (panel.parentElement !== element.parentElement || panel.nextElementSibling !== element) {
      element.parentElement.insertBefore(panel, element);
    }
    return true;
  }

  if (panel.parentElement !== element) {
    element.prepend(panel);
  } else if (element.firstElementChild !== panel) {
    element.prepend(panel);
  }
  return true;
}

function debugStatsMountTarget() {
  const context = currentCaptureContext();
  if (!context || typeof context.debugStatsMount !== "function") {
    return null;
  }

  const mount = context.debugStatsMount();
  let target = null;
  if (mount instanceof Element) {
    target = { element: mount, placement: "prepend" };
  } else if (mount && mount.element instanceof Element) {
    target = {
      element: mount.element,
      placement: mount.placement === "before" ? "before" : "prepend"
    };
  }

  return target && document.documentElement.contains(target.element) ? target : null;
}

function createDebugStatsHeader(stats) {
  const header = document.createElement("div");
  const title = document.createElement("div");
  const meta = document.createElement("div");
  const actions = document.createElement("div");

  header.className = "weblayer-debug-stats-header";
  header.dataset.weblayerUi = "true";
  title.className = "weblayer-debug-stats-title";
  title.dataset.weblayerUi = "true";
  title.textContent = stats.title || "WebLayer stats";
  actions.className = "weblayer-debug-stats-actions";
  actions.dataset.weblayerUi = "true";
  meta.className = "weblayer-debug-stats-meta";
  meta.dataset.weblayerUi = "true";
  meta.textContent = debugStatsMeta(stats);
  actions.append(meta);
  if (stats.dashboardUrl) {
    actions.append(createDebugStatsDashboardLink(stats.dashboardUrl));
  }

  header.append(title, actions);
  return header;
}

function createDebugStatsDashboardLink(url) {
  const link = document.createElement("a");
  link.className = "weblayer-debug-stats-link";
  link.dataset.weblayerUi = "true";
  link.href = url;
  link.target = "_blank";
  link.rel = "noopener noreferrer";
  link.textContent = "Dashboard";
  link.addEventListener("click", (event) => {
    event.stopPropagation();
  });
  return link;
}

function debugStatsMeta(stats) {
  const parts = [];
  if (stats.site) {
    parts.push(stats.site);
  }
  if (Number.isFinite(stats.generatedAtUnixMs)) {
    parts.push(shortTime(new Date(stats.generatedAtUnixMs)));
  }
  return parts.join(" | ");
}

function createDebugStatsSection(section) {
  const container = document.createElement("section");
  const title = document.createElement("div");
  const metrics = document.createElement("div");

  container.className = "weblayer-debug-stats-section";
  container.dataset.weblayerUi = "true";
  title.className = "weblayer-debug-stats-section-title";
  title.dataset.weblayerUi = "true";
  title.textContent = section.title || "";
  metrics.className = "weblayer-debug-stats-metrics";
  metrics.dataset.weblayerUi = "true";

  for (const metric of Array.isArray(section.metrics) ? section.metrics : []) {
    metrics.append(createDebugStatsMetric(metric));
  }

  container.append(title, metrics);
  return container;
}

function createDebugStatsMetric(metric) {
  const row = document.createElement("div");
  const label = document.createElement("div");
  const value = document.createElement("div");

  row.className = "weblayer-debug-stats-metric";
  row.dataset.weblayerUi = "true";
  label.className = "weblayer-debug-stats-label";
  label.dataset.weblayerUi = "true";
  label.textContent = metric.label || "";
  value.className = "weblayer-debug-stats-value";
  value.dataset.weblayerUi = "true";
  value.textContent = metric.value || "";

  row.append(label, value);
  if (metric.detail) {
    const detail = document.createElement("div");
    detail.className = "weblayer-debug-stats-detail";
    detail.dataset.weblayerUi = "true";
    detail.textContent = metric.detail;
    row.append(detail);
  }

  return row;
}

function handleWebLayerClick(event) {
  const target = eventTargetElement(event);
  const hiddenToggle = target ? target.closest(".weblayer-hidden-action") : null;
  if (hiddenToggle) {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
    const element = hiddenElementForToggle(hiddenToggle);
    if (element instanceof Element) {
      toggleHiddenElementExpanded(element);
    }
    return;
  }

  const button = target ? target.closest(".weblayer-feedback-button") : null;
  if (!button) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  event.stopImmediatePropagation();

  const clientId = button.dataset.weblayerClientId || "";
  const element = elementsByClientId.get(clientId);
  if (!element || button.disabled) {
    return;
  }

  void toggleFeedback(element, button);
}

function handleWebLayerPointerDown(event) {
  const target = eventTargetElement(event);
  const button = target ? target.closest(".weblayer-feedback-button") : null;
  if (!button || !button.classList.contains("weblayer-feedback-button--active")) {
    return;
  }

  button.dataset.weblayerSkipNextReasonBlur = "true";
  setTimeout(() => {
    delete button.dataset.weblayerSkipNextReasonBlur;
  }, 300);
}

function eventTargetElement(event) {
  if (event.target instanceof Element) {
    return event.target;
  }

  return event.target && event.target.parentElement instanceof Element
    ? event.target.parentElement
    : null;
}

async function toggleFeedback(element, button) {
  const wasActive = button.classList.contains("weblayer-feedback-button--active");

  if (!wasActive) {
    markKeepVisibleAfterFeedback(element);
    button.dataset.weblayerFeedbackPersisted = "false";
    setFeedbackButtonActive(button, true);
    showFeedbackReasonPanel(element, button);
    return;
  }

  const panel = wasActive
    ? feedbackReasonPanel(element, button.dataset.weblayerClientId || "")
    : null;
  const persisted = button.dataset.weblayerFeedbackPersisted === "true";
  const reason = currentFeedbackReason(element, button);

  if (panel) {
    cancelScheduledReasonUpdate(panel);
    panel.dataset.weblayerClosing = "true";
  }

  button.disabled = true;
  button.dataset.weblayerFeedbackState = "pending";

  try {
    if (persisted) {
      const response = await sendFeedbackEvent(element, "undoThumbsDown", reason, button);

      if (!response || !response.ok) {
        throw new Error(response && response.error ? response.error : "Daemon request failed.");
      }

      applyCommands(response.commands || []);
    }

    clearKeepVisibleAfterFeedback(element);
    delete button.dataset.weblayerFeedbackPersisted;
    setFeedbackButtonActive(button, false);
    removeFeedbackReasonPanel(element, button.dataset.weblayerClientId || "");
  } catch (error) {
    if (panel) {
      delete panel.dataset.weblayerClosing;
      setFeedbackSaveStatus(panel, "Undo failed", "error");
    }
    button.dataset.weblayerFeedbackState = wasActive ? "active" : "unavailable";
    button.title = `WebLayer feedback unavailable: ${
      error instanceof Error ? error.message : String(error)
    }`;
  } finally {
    button.disabled = false;
  }
}

async function sendFeedbackEvent(element, feedback, reason, button) {
  const snapshot = snapshotElement(element);
  if (!snapshot) {
    return null;
  }
  const feedbackContextId = currentFeedbackContextId(element, snapshot.clientId, button);
  const message = {
    type: "weblayer:feedback",
    feedback,
    reason,
    page: pageSnapshot(),
    element: snapshotForMessage(snapshot),
    feedbackContextId
  };

  return sendMessage(message);
}

function setFeedbackButtonActive(button, active) {
  const label = active ? "Undo thumbs-down feedback" : "Hide this post";
  button.classList.toggle("weblayer-feedback-button--active", active);
  button.dataset.weblayerFeedbackState = active ? "active" : "idle";
  button.title = label;
  button.setAttribute("aria-label", label);
  button.setAttribute("aria-pressed", active ? "true" : "false");
}

function markKeepVisibleAfterFeedback(element) {
  const snapshot = snapshotElement(element);
  if (snapshot && snapshot.snapshotHash) {
    element.dataset.weblayerKeepVisibleAfterFeedbackHash = snapshot.snapshotHash;
  }
}

function clearKeepVisibleAfterFeedback(element) {
  delete element.dataset.weblayerKeepVisibleAfterFeedbackHash;
}

function shouldKeepVisibleAfterFeedback(element, command) {
  if (hasActiveFeedbackSession(element)) {
    return true;
  }

  const keepVisibleHash = element.dataset.weblayerKeepVisibleAfterFeedbackHash;
  if (!keepVisibleHash) {
    return false;
  }

  const commandHash = command.target && command.target.mustMatchSnapshotHash;
  if (commandHash) {
    return commandHash === keepVisibleHash;
  }

  const snapshot = snapshotElement(element);
  return snapshot && snapshot.snapshotHash === keepVisibleHash;
}

function hasActiveFeedbackSession(element) {
  return (
    element.querySelector(".weblayer-feedback-panel") !== null ||
    element.querySelector(".weblayer-feedback-button--active") !== null
  );
}

function showFeedbackReasonPanel(element, button) {
  const clientId = button.dataset.weblayerClientId || "";
  if (!clientId) {
    return;
  }

  const existingPanel = feedbackReasonPanel(element, clientId);
  if (existingPanel) {
    const input = existingPanel.querySelector(".weblayer-feedback-reason-input");
    if (input instanceof HTMLElement) {
      input.focus();
    }
    return;
  }

  const actionBar = findActionBar(element);
  if (!actionBar) {
    return;
  }

  const panel = createFeedbackReasonPanel(element, button);
  actionBar.insertAdjacentElement("afterend", panel);

  const input = panel.querySelector(".weblayer-feedback-reason-input");
  if (input instanceof HTMLElement) {
    input.focus();
  }
}

function createFeedbackReasonPanel(element, button) {
  const clientId = button.dataset.weblayerClientId || "";
  const panel = document.createElement("div");
  const heading = document.createElement("div");
  const label = document.createElement("div");
  const status = document.createElement("div");
  const chips = document.createElement("div");
  const input = document.createElement("textarea");

  panel.className = "weblayer-feedback-panel";
  panel.dataset.weblayerUi = "true";
  panel.dataset.weblayerClientId = clientId;
  panel.dataset.weblayerSavedReason = "";
  panel.dataset.weblayerSaveState = "idle";

  heading.className = "weblayer-feedback-panel-heading";
  heading.dataset.weblayerUi = "true";

  label.className = "weblayer-feedback-panel-label";
  label.dataset.weblayerUi = "true";
  label.textContent = "Reason";

  status.className = "weblayer-feedback-save-status";
  status.dataset.weblayerUi = "true";
  status.setAttribute("role", "status");
  status.setAttribute("aria-live", "polite");
  status.textContent = "Add a reason";

  chips.className = "weblayer-feedback-reason-chips";
  chips.dataset.weblayerUi = "true";
  for (const reason of FEEDBACK_REASON_PRESETS) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "weblayer-feedback-reason-chip";
    chip.dataset.weblayerUi = "true";
    chip.dataset.weblayerReason = reason;
    chip.textContent = reason;
    chip.setAttribute("aria-pressed", "false");
    chip.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      event.stopImmediatePropagation();
      input.value = reason;
      input.focus();
      updateSelectedReasonChip(panel, reason);
      scheduleReasonUpdate(element, button, panel, { immediate: true });
    });
    chips.append(chip);
  }

  input.className = "weblayer-feedback-reason-input";
  input.dataset.weblayerUi = "true";
  input.rows = 2;
  input.placeholder = "Add a reason";
  input.addEventListener("click", (event) => {
    event.stopPropagation();
  });
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
  });
  input.addEventListener("input", () => {
    updateSelectedReasonChip(panel, currentPanelReason(panel));
    scheduleReasonUpdate(element, button, panel);
  });
  input.addEventListener("blur", () => {
    if (button.dataset.weblayerSkipNextReasonBlur === "true") {
      return;
    }

    scheduleReasonUpdate(element, button, panel, { immediate: true });
  });
  input.addEventListener("change", () => {
    scheduleReasonUpdate(element, button, panel, { immediate: true });
  });

  panel.addEventListener("pointerdown", stopPanelEvent);
  panel.addEventListener("click", stopPanelEvent);
  panel.addEventListener("keydown", stopPanelEvent);

  heading.append(label, status);
  panel.append(heading, chips, input);
  return panel;
}

function stopPanelEvent(event) {
  event.stopPropagation();
}

function scheduleReasonUpdate(element, button, panel, options = {}) {
  if (
    !button.classList.contains("weblayer-feedback-button--active") ||
    panel.dataset.weblayerClosing === "true"
  ) {
    return;
  }

  const reason = currentPanelReason(panel);
  updateSelectedReasonChip(panel, reason);
  cancelScheduledReasonUpdate(panel);

  if (!reason) {
    setFeedbackSaveStatus(panel, "Add a reason", "idle");
    return;
  }

  if ((panel.dataset.weblayerSavedReason || "") === reason) {
    setFeedbackSaveStatus(panel, "Saved", "saved");
    return;
  }

  setFeedbackSaveStatus(panel, "Saving...", "saving");

  const save = () => {
    feedbackReasonTimers.delete(panel);
    void sendReasonUpdate(element, button, panel, reason);
  };

  if (options.immediate) {
    save();
    return;
  }

  feedbackReasonTimers.set(
    panel,
    setTimeout(save, FEEDBACK_REASON_SAVE_DEBOUNCE_MS)
  );
}

function cancelScheduledReasonUpdate(panel) {
  const timer = feedbackReasonTimers.get(panel);
  if (timer) {
    clearTimeout(timer);
    feedbackReasonTimers.delete(panel);
  }
}

async function sendReasonUpdate(element, button, panel, reason) {
  if (panel.dataset.weblayerClosing === "true" || !reason) {
    return;
  }

  const requestId = String(nextFeedbackSaveId);
  nextFeedbackSaveId += 1;
  const previousTitle = button.title;
  const feedback = button.dataset.weblayerFeedbackPersisted === "true"
    ? "updateReason"
    : "thumbsDown";
  panel.dataset.weblayerSaveRequestId = requestId;
  button.disabled = true;
  button.dataset.weblayerFeedbackState = "pending";
  setFeedbackSaveStatus(panel, "Saving...", "saving");

  try {
    const response = await sendFeedbackEvent(element, feedback, reason, button);
    if (!response || !response.ok) {
      throw new Error(response && response.error ? response.error : "Daemon request failed.");
    }

    applyCommands(response.commands || []);
    if (!panel.isConnected || panel.dataset.weblayerClosing === "true") {
      return;
    }
    if (
      panel.dataset.weblayerSaveRequestId !== requestId ||
      currentPanelReason(panel) !== reason
    ) {
      return;
    }

    panel.dataset.weblayerSavedReason = reason;
    button.dataset.weblayerFeedbackPersisted = "true";
    setFeedbackSaveStatus(panel, `Saved ${shortTime(new Date())}`, "saved");
    if (button.classList.contains("weblayer-feedback-button--active")) {
      button.dataset.weblayerFeedbackState = "active";
    }
  } catch (error) {
    if (
      panel.isConnected &&
      panel.dataset.weblayerSaveRequestId === requestId &&
      panel.dataset.weblayerClosing !== "true"
    ) {
      setFeedbackSaveStatus(panel, "Save failed", "error");
    }

    button.dataset.weblayerFeedbackState = "active";
    button.title = `WebLayer feedback unavailable: ${
      error instanceof Error ? error.message : String(error)
    }`;
    setTimeout(() => {
      if (button.dataset.weblayerFeedbackState === "active") {
        button.title = previousTitle;
      }
    }, 2500);
  } finally {
    if (
      panel.isConnected &&
      panel.dataset.weblayerSaveRequestId === requestId &&
      panel.dataset.weblayerClosing !== "true"
    ) {
      button.disabled = false;
    }
  }
}

function currentPanelReason(panel) {
  const input = panel.querySelector(".weblayer-feedback-reason-input");
  return input instanceof HTMLTextAreaElement ? input.value.trim() : "";
}

function updateSelectedReasonChip(panel, reason) {
  for (const chip of panel.querySelectorAll(".weblayer-feedback-reason-chip")) {
    const selected = chip.dataset.weblayerReason === reason;
    chip.classList.toggle("weblayer-feedback-reason-chip--selected", selected);
    chip.setAttribute("aria-pressed", selected ? "true" : "false");
  }
}

function setFeedbackSaveStatus(panel, text, state) {
  panel.dataset.weblayerSaveState = state;
  const status = panel.querySelector(".weblayer-feedback-save-status");
  if (status) {
    status.textContent = text;
  }
}

function shortTime(date) {
  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit"
  });
}

function currentFeedbackReason(element, button) {
  const clientId = button.dataset.weblayerClientId || "";
  const panel = feedbackReasonPanel(element, clientId);
  const input = panel && panel.querySelector(".weblayer-feedback-reason-input");
  return input instanceof HTMLTextAreaElement ? input.value.trim() : "";
}

function feedbackReasonPanel(element, clientId) {
  if (!clientId) {
    return null;
  }

  return element.querySelector(
    `.weblayer-feedback-panel[data-weblayer-client-id="${cssEscape(clientId)}"]`
  );
}

function removeFeedbackReasonPanel(element, clientId) {
  const panel = feedbackReasonPanel(element, clientId);
  if (panel) {
    cancelScheduledReasonUpdate(panel);
    panel.remove();
  }
}

function resolveTarget(target) {
  if (!target || typeof target !== "object") {
    return null;
  }

  if (target.clientId && elementsByClientId.has(target.clientId)) {
    return elementsByClientId.get(target.clientId);
  }

  if (target.selector) {
    try {
      const element = document.querySelector(target.selector);
      if (element) {
        return element;
      }
    } catch (_error) {
      return null;
    }
  }

  return null;
}

function targetStillMatches(element, target) {
  if (!target || !target.mustMatchSnapshotHash) {
    return true;
  }

  const snapshot = snapshotElement(element);
  return snapshot && snapshot.snapshotHash === target.mustMatchSnapshotHash;
}

function clearWebLayerChanges(element) {
  element.classList.remove(
    "weblayer-hidden",
    "weblayer-hidden--expanded",
    "weblayer-dimmed",
    "weblayer-replaced"
  );

  const placeholder = element.querySelector(":scope > .weblayer-hidden-placeholder");
  if (placeholder) {
    placeholder.remove();
  }

  const hiddenId = element.dataset.weblayerHiddenId || "";
  const siblingPlaceholder = hiddenId ? hiddenPlaceholderForId(hiddenId) : null;
  if (siblingPlaceholder) {
    siblingPlaceholder.remove();
  }
  delete element.dataset.weblayerHiddenId;

  const hiddenContent = element.querySelector(":scope > .weblayer-hidden-content");
  if (hiddenContent) {
    while (hiddenContent.firstChild) {
      element.insertBefore(hiddenContent.firstChild, hiddenContent);
    }
    hiddenContent.remove();
  }

  const badge = element.querySelector(":scope > .weblayer-badge");
  if (badge) {
    badge.remove();
  }

  if (element.dataset.weblayerOriginalText) {
    element.innerText = element.dataset.weblayerOriginalText;
    delete element.dataset.weblayerOriginalText;
  }
}

function hideContainerForElement(element) {
  const xPost = element.closest("article[data-testid='tweet']");
  if (xPost instanceof Element) {
    // Keep X timeline cells mounted so virtualized lists retain their structure.
    // The placeholder is inserted next to the article inside the same cell.
    return xPost;
  }

  const post = element.closest("article, [role='article']");
  return post instanceof Element ? post : element;
}

function collapseHiddenElement(element, command, options = {}) {
  const wasExpanded = options.expanded === true;
  element.classList.add("weblayer-hidden");
  element.classList.toggle("weblayer-hidden--expanded", wasExpanded);

  const placeholder = ensureHiddenPlaceholderSibling(element);
  updateHiddenPlaceholderExpandedState(placeholder, wasExpanded);

  placeholder.replaceChildren(createHiddenToggle(element, command));
}

function ensureHiddenPlaceholderSibling(element) {
  const hiddenId = hiddenIdForElement(element);
  let placeholder = hiddenPlaceholderForId(hiddenId);
  if (!placeholder) {
    placeholder = document.createElement("div");
    placeholder.className = "weblayer-hidden-placeholder";
    placeholder.dataset.weblayerUi = "true";
    placeholder.dataset.weblayerHiddenFor = hiddenId;
  }

  const parent = element.parentElement;
  if (parent && (placeholder.parentElement !== parent || placeholder.nextElementSibling !== element)) {
    parent.insertBefore(placeholder, element);
  } else if (!parent && placeholder.parentElement !== element) {
    element.prepend(placeholder);
  }

  return placeholder;
}

function hiddenIdForElement(element) {
  if (!element.dataset.weblayerHiddenId) {
    element.dataset.weblayerHiddenId = `weblayer-hidden-${nextHiddenId}`;
    nextHiddenId += 1;
  }
  return element.dataset.weblayerHiddenId;
}

function hiddenPlaceholderForId(hiddenId) {
  return document.querySelector(
    `.weblayer-hidden-placeholder[data-weblayer-hidden-for="${cssEscape(hiddenId)}"]`
  );
}

function hiddenElementForToggle(toggle) {
  const placeholder = toggle.closest(".weblayer-hidden-placeholder");
  const hiddenId = placeholder && placeholder.dataset.weblayerHiddenFor;
  if (hiddenId) {
    return document.querySelector(`[data-weblayer-hidden-id="${cssEscape(hiddenId)}"]`);
  }

  return toggle.closest(".weblayer-hidden");
}

function createHiddenToggle(element, command) {
  const toggle = document.createElement("div");
  const title = document.createElement("span");
  const detail = document.createElement("span");
  const action = document.createElement("button");
  const expanded = element.classList.contains("weblayer-hidden--expanded");
  const reason = hiddenDetailText(command);

  toggle.className = "weblayer-hidden-toggle";
  toggle.dataset.weblayerUi = "true";

  title.className = "weblayer-hidden-title";
  title.dataset.weblayerUi = "true";
  title.textContent = "Hidden by WebLayer";

  detail.className = "weblayer-hidden-detail";
  detail.dataset.weblayerUi = "true";
  detail.dataset.weblayerHiddenReason = reason;
  detail.textContent = expanded ? expandedHiddenDetailText(reason) : reason;

  action.className = "weblayer-hidden-action";
  action.dataset.weblayerUi = "true";
  action.type = "button";
  action.setAttribute("aria-expanded", expanded ? "true" : "false");
  action.title = expanded ? "Hide this WebLayer-hidden post again" : "Show WebLayer-hidden post";
  action.textContent = expanded ? "Hide" : "Show";

  toggle.append(title, detail, action);
  return toggle;
}

function toggleHiddenElementExpanded(element) {
  element.classList.toggle("weblayer-hidden--expanded");
  const hiddenId = element.dataset.weblayerHiddenId || "";
  const placeholder = hiddenId ? hiddenPlaceholderForId(hiddenId) : null;
  const toggle = placeholder && placeholder.querySelector(".weblayer-hidden-toggle");
  if (!toggle) {
    return;
  }

  const expanded = element.classList.contains("weblayer-hidden--expanded");
  updateHiddenPlaceholderExpandedState(placeholder, expanded);
  const detail = toggle.querySelector(".weblayer-hidden-detail");
  const action = toggle.querySelector(".weblayer-hidden-action");
  if (detail) {
    const originalDetail = detail.dataset.weblayerHiddenReason || detail.textContent || "";
    detail.dataset.weblayerHiddenReason = originalDetail;
    detail.textContent = expanded && originalDetail
      ? expandedHiddenDetailText(originalDetail)
      : originalDetail;
  }
  if (action) {
    action.setAttribute("aria-expanded", expanded ? "true" : "false");
    action.title = expanded ? "Hide this WebLayer-hidden post again" : "Show WebLayer-hidden post";
    action.textContent = expanded ? "Hide" : "Show";
  }
}

function hiddenDetailText(command) {
  const value = command.reason || command.label || "";
  return value ? String(value) : "Click to show the post.";
}

function expandedHiddenDetailText(detail) {
  return detail ? `Shown now - ${detail}` : "Shown now.";
}

function updateHiddenPlaceholderExpandedState(placeholder, expanded) {
  placeholder.classList.toggle("weblayer-hidden-placeholder--expanded", expanded);
}

function replaceRegionText(element, replacementText) {
  element.dataset.weblayerOriginalText = element.innerText;
  element.innerText = replacementText;
  element.classList.add("weblayer-replaced");
}

function insertBadge(element, command) {
  const badgeText = command.label || command.reason || "WebLayer";
  const badge = document.createElement("div");
  const text = document.createElement("span");

  badge.className = "weblayer-badge";
  badge.dataset.weblayerUi = "true";

  text.className = "weblayer-badge-text";
  text.dataset.weblayerUi = "true";
  text.textContent = badgeText;

  badge.append(text);
  element.prepend(badge);
}

function installOptimisticFeedbackControl(element, snapshot) {
  insertFeedbackControl(
    element,
    {
      target: { clientId: snapshot.clientId },
      label: "Hide this post"
    },
    { allowPendingContext: true }
  );
}

function insertFeedbackControl(element, command, options = {}) {
  const clientId = command.target && command.target.clientId
    ? command.target.clientId
    : getClientId(element);
  const isSubjectPost = isSubjectPostElement(element, clientId);
  const label = command.label || "Hide this post";
  const allowPendingContext = options.allowPendingContext === true;
  const existingButton = element.querySelector(
    `.weblayer-feedback-button[data-weblayer-client-id="${cssEscape(clientId)}"]`
  );
  if (existingButton) {
    existingButton.classList.toggle("weblayer-feedback-button--subject", isSubjectPost);
    if (command.feedbackContextId) {
      storeFeedbackContextId(existingButton, command.feedbackContextId);
      setFeedbackButtonReady(existingButton, label);
    } else if (allowPendingContext && !existingButton.dataset.weblayerFeedbackContextId) {
      setFeedbackButtonWaitingForContext(existingButton);
    } else if (!allowPendingContext) {
      storeFeedbackContextId(existingButton, command.feedbackContextId);
    }
    if (!existingButton.hasAttribute("aria-pressed")) {
      existingButton.setAttribute("aria-pressed", "false");
    }
    return;
  }

  const actionBar = findActionBar(element);
  if (!actionBar) {
    return;
  }

  const likeSlot = findActionSlot(actionBar, "[data-testid='like'], [data-testid='unlike']");
  const slot = createFeedbackSlot(likeSlot || actionBar.firstElementChild);
  slot.dataset.weblayerUi = "true";
  slot.append(
    createFeedbackButton(
      clientId,
      label,
      isSubjectPost,
      command.feedbackContextId,
      { allowPendingContext }
    )
  );

  if (likeSlot && likeSlot.parentElement === actionBar && likeSlot.nextSibling) {
    actionBar.insertBefore(slot, likeSlot.nextSibling);
  } else {
    actionBar.append(slot);
  }
}

function createFeedbackSlot(referenceSlot) {
  const slot = document.createElement(
    referenceSlot && referenceSlot.tagName
      ? referenceSlot.tagName.toLowerCase()
      : "div"
  );
  const referenceClass = referenceSlot && typeof referenceSlot.className === "string"
    ? referenceSlot.className
    : "";
  slot.className = referenceClass
    ? `${referenceClass} weblayer-feedback-slot`
    : "weblayer-feedback-slot";
  return slot;
}

function createFeedbackButton(
  clientId,
  label,
  isSubjectPost,
  feedbackContextId,
  options = {}
) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "weblayer-feedback-button";
  button.classList.toggle("weblayer-feedback-button--subject", isSubjectPost);
  button.dataset.weblayerUi = "true";
  button.dataset.weblayerClientId = clientId;
  button.dataset.weblayerFeedback = "thumbsDown";
  button.dataset.weblayerFeedbackState = "idle";
  button.title = label;
  button.setAttribute("aria-label", label);
  button.setAttribute("aria-pressed", "false");
  if (feedbackContextId) {
    storeFeedbackContextId(button, feedbackContextId);
    setFeedbackButtonReady(button, label);
  } else if (options.allowPendingContext === true) {
    setFeedbackButtonWaitingForContext(button);
  } else {
    storeFeedbackContextId(button, feedbackContextId);
  }
  button.append(createThumbsDownIcon());
  return button;
}

function storeFeedbackContextId(button, feedbackContextId) {
  if (!feedbackContextId) {
    throw new Error("Feedback context ID is required.");
  }
  button.dataset.weblayerFeedbackContextId = feedbackContextId;
}

function setFeedbackButtonReady(button, label) {
  if (button.dataset.weblayerFeedbackState === "pending") {
    return;
  }

  button.disabled = false;
  button.removeAttribute("aria-disabled");

  if (button.classList.contains("weblayer-feedback-button--active")) {
    setFeedbackButtonActive(button, true);
    return;
  }

  button.dataset.weblayerFeedbackState = "idle";
  button.title = label || "Hide this post";
  button.setAttribute("aria-label", label || "Hide this post");
  button.setAttribute("aria-pressed", "false");
}

function setFeedbackButtonWaitingForContext(button) {
  if (button.dataset.weblayerFeedbackState === "pending") {
    return;
  }

  button.disabled = true;
  button.dataset.weblayerFeedbackState = "contextPending";
  button.title = "Preparing WebLayer feedback";
  button.setAttribute("aria-label", "Preparing WebLayer feedback");
  button.setAttribute("aria-disabled", "true");
  button.setAttribute("aria-pressed", "false");
}

function currentFeedbackContextId(element, clientId, button) {
  const source = button || element.querySelector(
    `.weblayer-feedback-button[data-weblayer-client-id="${cssEscape(clientId)}"]`
  );
  const feedbackContextId = source && source.dataset.weblayerFeedbackContextId
    ? source.dataset.weblayerFeedbackContextId
    : "";
  if (!feedbackContextId) {
    throw new Error("Feedback context ID is missing.");
  }
  return feedbackContextId;
}

function isSubjectPostElement(element, clientId) {
  const pageStatusId = statusIdFromUrl(location.href);
  if (!pageStatusId) {
    return false;
  }

  const snapshot = snapshotsByClientId.get(clientId);
  if (snapshot && snapshot.links.some((link) => statusIdFromUrl(link.href) === pageStatusId)) {
    return true;
  }

  const postRoot = element.closest("article, [role='article'], [data-testid='tweet']");
  const firstPost = firstVisiblePostInMain();
  return postRoot !== null && postRoot === firstPost;
}

function firstVisiblePostInMain() {
  const main = document.querySelector("main");
  if (!main) {
    return null;
  }

  return Array.from(main.querySelectorAll("article, [role='article'], [data-testid='tweet']"))
    .filter(isVisibleRegion)[0] || null;
}

function statusIdFromUrl(value) {
  const match = String(value || "").match(/\/status\/(\d+)/);
  return match ? match[1] : null;
}

function createThumbsDownIcon() {
  const namespace = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(namespace, "svg");
  const path = document.createElementNS(namespace, "path");

  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("class", "weblayer-feedback-icon");
  path.setAttribute(
    "d",
    "M10 15v4a3 3 0 0 0 3 3l4-9V2H5.7a2 2 0 0 0-2 1.7l-1.4 9A2 2 0 0 0 4.3 15H10Zm7-13h2.7A2.3 2.3 0 0 1 22 4.3v6.4a2.3 2.3 0 0 1-2.3 2.3H17V2Z"
  );
  svg.append(path);
  return svg;
}

function findActionBar(element) {
  const candidates = Array.from(element.querySelectorAll("[role='group']"))
    .filter(isVisibleRegion)
    .filter((candidate) => candidate.querySelectorAll("button, [role='button']").length >= 2)
    .sort((left, right) => {
      const leftRect = left.getBoundingClientRect();
      const rightRect = right.getBoundingClientRect();
      return rightRect.top - leftRect.top;
    });

  return candidates[0] || null;
}

function findActionSlot(actionBar, selector) {
  const control = actionBar.querySelector(selector);
  if (!control) {
    return null;
  }

  let current = control;
  while (current.parentElement && current.parentElement !== actionBar) {
    current = current.parentElement;
  }

  return current.parentElement === actionBar ? current : null;
}

function markBatchUnavailable(batch, error) {
  for (const snapshot of batch) {
    const element = elementsByClientId.get(snapshot.clientId);
    if (!element) {
      continue;
    }

    element.dataset.weblayerState = "unavailable";
    element.title = `WebLayer daemon unavailable: ${
      error instanceof Error ? error.message : String(error)
    }`;
    markFeedbackControlUnavailable(element, snapshot.clientId, error);
  }
}

function markFeedbackControlUnavailable(element, clientId, error) {
  const button = element.querySelector(
    `.weblayer-feedback-button[data-weblayer-client-id="${cssEscape(clientId)}"]`
  );
  if (!button || button.classList.contains("weblayer-feedback-button--active")) {
    return;
  }

  const message = `WebLayer daemon unavailable: ${
    error instanceof Error ? error.message : String(error)
  }`;
  button.disabled = true;
  button.dataset.weblayerFeedbackState = "unavailable";
  button.title = message;
  button.setAttribute("aria-label", message);
  button.setAttribute("aria-disabled", "true");
  button.setAttribute("aria-pressed", "false");
}

function cssPath(element) {
  const parts = [];
  let current = element;

  while (current && current.nodeType === Node.ELEMENT_NODE && current !== document.documentElement) {
    const tag = current.tagName.toLowerCase();
    const id = current.getAttribute("id");
    if (id) {
      parts.unshift(`${tag}#${cssEscape(id)}`);
      break;
    }

    let index = 1;
    let sibling = current.previousElementSibling;
    while (sibling) {
      if (sibling.tagName === current.tagName) {
        index += 1;
      }
      sibling = sibling.previousElementSibling;
    }

    parts.unshift(`${tag}:nth-of-type(${index})`);
    current = current.parentElement;
  }

  return parts.length > 0 ? parts.join(" > ") : null;
}

function cssEscape(value) {
  if (window.CSS && typeof window.CSS.escape === "function") {
    return window.CSS.escape(value);
  }

  return String(value).replace(/[^a-zA-Z0-9_-]/g, "\\$&");
}

function absoluteUrl(value) {
  if (!value) {
    return "";
  }

  try {
    return new URL(value, location.href).href;
  } catch (_error) {
    return "";
  }
}

function normalizeText(text) {
  return String(text || "").replace(/\s+/g, " ").trim();
}

function truncate(value, maxLength) {
  const stringValue = String(value || "");
  return stringValue.length > maxLength ? stringValue.slice(0, maxLength) : stringValue;
}

function stringOrNull(value) {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function stableHash(value) {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

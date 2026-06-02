(() => {
  const X_COM_HOSTS = new Set(["x.com", "www.x.com", "twitter.com", "www.twitter.com"]);
  const X_COM_POST_SELECTOR = "article[data-testid='tweet']";
  const X_COM_STATUS_PATH = /^\/[A-Za-z0-9_]{1,15}\/status\/\d+(?:\/.*)?$/;

  function current(locationValue = window.location, root = document) {
    let url;
    try {
      url = new URL(locationValue.href);
    } catch (_error) {
      return null;
    }

    const xComPageKind = supportedXComPageKind(url);
    if (!xComPageKind) {
      return null;
    }

    return {
      id: "x.com",
      pageKind: xComPageKind,
      key: `x.com:${xComPageKind}:${url.origin}${url.pathname}${url.search}`,
      collectCandidates: () => collectXComCandidates(root),
      isSupportedElement: isSupportedXComElement,
      debugStatsMount: () => xComDebugStatsMount(root),
      metadataForElement: (element, snapshot) => xComMetadataForElement(
        root,
        url,
        xComPageKind,
        element,
        snapshot
      )
    };
  }

  function supportedXComPageKind(url) {
    if (!X_COM_HOSTS.has(url.hostname.toLowerCase())) {
      return null;
    }

    const path = normalizedPath(url.pathname);
    if (path === "/home") {
      return "homeTimeline";
    }
    if (path === "/search") {
      return "searchResults";
    }
    if (path === "/explore") {
      return "exploreTimeline";
    }
    if (X_COM_STATUS_PATH.test(path)) {
      return "statusThread";
    }

    return null;
  }

  function collectXComCandidates(root) {
    return Array.from(root.querySelectorAll(`main ${X_COM_POST_SELECTOR}`));
  }

  function xComDebugStatsMount(root) {
    const sidebar = root.querySelector("[data-testid='sidebarColumn']");
    if (sidebar instanceof Element && isVisibleElement(sidebar)) {
      return xComSidebarContentMount(sidebar) || sidebar;
    }

    for (const element of root.querySelectorAll("aside, [role='complementary']")) {
      if (element instanceof Element && isVisibleElement(element)) {
        return xComSidebarContentMount(element) || element;
      }
    }

    return null;
  }

  function xComSidebarContentMount(sidebar) {
    const timeline = sidebar.querySelector("[aria-label^='Timeline:']");
    if (timeline instanceof Element && isVisibleElement(timeline)) {
      return timeline;
    }

    for (const child of sidebar.children) {
      if (
        child instanceof Element &&
        !child.matches("[data-weblayer-ui='true']") &&
        isVisibleElement(child)
      ) {
        return child;
      }
    }

    return null;
  }

  function xComMetadataForElement(root, url, pageKind, element, snapshot) {
    const visiblePosts = collectXComCandidates(root).filter(isSupportedXComElement);
    const visibleIndex = visiblePosts.indexOf(element);
    const postId = statusIdFromLinks(snapshot.links);
    const pagePostId = statusIdFromUrl(url.href);
    const effectivePostId = postId || (
      pageKind === "statusThread" && visibleIndex === 0 ? pagePostId : null
    );

    const authorHandle = authorHandleFromLinks(snapshot.links);

    return {
      xCom: {
        pageKind,
        postId: effectivePostId,
        authorHandle,
        postText: xComPostTextFromElement(element, snapshot, authorHandle),
        visibleIndex: visibleIndex >= 0 ? visibleIndex : null,
        replyingToHandles: replyingToHandlesFromText(snapshot.text)
      }
    };
  }

  function xComPostTextFromElement(element, snapshot, authorHandle) {
    const textNode = Array.from(element.querySelectorAll("[data-testid='tweetText']"))
      .find((node) => node.closest(X_COM_POST_SELECTOR) === element);
    const text = textNode ? normalizeText(textNode.innerText || textNode.textContent || "") : "";

    return text || xComPostTextFromSnapshot(snapshot && snapshot.text, authorHandle);
  }

  function xComPostTextFromSnapshot(text, authorHandle) {
    let value = normalizeText(text).replace(/\s*·\s*/g, " · ");
    if (!value) {
      return null;
    }

    value = stripXStatusMetadataSuffix(value);

    const handle = normalizeText(authorHandle).toLowerCase();
    if (handle) {
      const handleIndex = value.toLowerCase().indexOf(handle);
      if (handleIndex >= 0 && handleIndex <= 80) {
        value = value.slice(handleIndex + handle.length).trim();
      }
    }

    value = value
      .replace(/^(?:·\s*)?(?:(?:now|\d+[smhd])|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2})\s*/i, "")
      .replace(/\s*·\s*/g, " · ")
      .replace(/\s+(?:\d+(?:\.\d+)?[KMB]?){2,}$/i, "")
      .replace(/([^\d\s])(?:\d+(?:\.\d+)?[KMB]?){2,}$/i, "$1")
      .trim();

    return value || null;
  }

  function stripXStatusMetadataSuffix(text) {
    const month = "(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\\.?";
    const date = `${month}\\s+\\d{1,2},\\s+\\d{4}`;
    const time = "\\d{1,2}:\\d{2}\\s*(?:AM|PM)";
    const views = "[\\d,.]+\\s*[KMB]?\\s+Views?";
    const suffixes = [
      new RegExp(`\\s*${time}\\s*·\\s*${date}\\s*·\\s*${views}$`, "i"),
      new RegExp(`\\s*${date}\\s*·\\s*${views}$`, "i"),
      new RegExp(`\\s*${time}\\s*·\\s*${date}$`, "i"),
      new RegExp(`\\s*·\\s*${views}$`, "i")
    ];

    return suffixes.reduce((value, pattern) => value.replace(pattern, ""), text).trim();
  }

  function statusIdFromLinks(links) {
    for (const link of Array.isArray(links) ? links : []) {
      const statusId = statusIdFromUrl(link.href);
      if (statusId) {
        return statusId;
      }
    }

    return null;
  }

  function statusIdFromUrl(value) {
    const match = String(value || "").match(/\/status\/(\d+)/);
    return match ? match[1] : null;
  }

  function authorHandleFromLinks(links) {
    for (const link of Array.isArray(links) ? links : []) {
      const handle = authorHandleFromStatusUrl(link.href);
      if (handle) {
        return handle;
      }
    }

    return null;
  }

  function authorHandleFromStatusUrl(value) {
    let url;
    try {
      url = new URL(value);
    } catch (_error) {
      return null;
    }

    const match = url.pathname.match(/^\/([A-Za-z0-9_]{1,15})\/status\/\d+/);
    return match ? `@${match[1]}` : null;
  }

  function replyingToHandlesFromText(text) {
    const match = String(text || "").match(
      /Replying to\s+((?:@[A-Za-z0-9_]{1,15}(?:\s*(?:,|and)\s*)?)+)/i
    );
    if (!match) {
      return [];
    }

    return uniqueStrings(
      Array.from(match[1].matchAll(/@[A-Za-z0-9_]{1,15}/g)).map((handle) => handle[0])
    );
  }

  function uniqueStrings(values) {
    const seen = new Set();
    const unique = [];
    for (const value of values) {
      if (!seen.has(value)) {
        seen.add(value);
        unique.push(value);
      }
    }

    return unique;
  }

  function isSupportedXComElement(element) {
    return (
      element instanceof Element &&
      element.matches(X_COM_POST_SELECTOR) &&
      element.closest("main") !== null &&
      !hasAncestorPost(element)
    );
  }

  function hasAncestorPost(element) {
    return (
      element.parentElement !== null &&
      element.parentElement.closest(X_COM_POST_SELECTOR) !== null
    );
  }

  function isVisibleElement(element) {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return (
      rect.width >= 160 &&
      rect.height >= 1 &&
      style.display !== "none" &&
      style.visibility !== "hidden" &&
      style.opacity !== "0"
    );
  }

  function normalizedPath(pathname) {
    const path = String(pathname || "/").replace(/\/+$/, "");
    return path || "/";
  }

  function normalizeText(text) {
    return String(text || "").replace(/\s+/g, " ").trim();
  }

  window.WebLayerSiteAdapters = { current };
})();

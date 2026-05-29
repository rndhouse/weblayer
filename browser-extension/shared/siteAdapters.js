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

  function xComMetadataForElement(root, url, pageKind, element, snapshot) {
    const visiblePosts = collectXComCandidates(root).filter(isSupportedXComElement);
    const visibleIndex = visiblePosts.indexOf(element);
    const postId = statusIdFromLinks(snapshot.links);
    const pagePostId = statusIdFromUrl(url.href);
    const effectivePostId = postId || (
      pageKind === "statusThread" && visibleIndex === 0 ? pagePostId : null
    );

    return {
      xCom: {
        pageKind,
        postId: effectivePostId,
        authorHandle: authorHandleFromLinks(snapshot.links),
        visibleIndex: visibleIndex >= 0 ? visibleIndex : null,
        replyingToHandles: replyingToHandlesFromText(snapshot.text)
      }
    };
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

  function normalizedPath(pathname) {
    const path = String(pathname || "/").replace(/\/+$/, "");
    return path || "/";
  }

  window.WebLayerSiteAdapters = { current };
})();

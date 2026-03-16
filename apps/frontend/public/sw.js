const SHELL_CACHE = "archivist-shell-v2";
const ASSET_CACHE = "archivist-assets-v2";
const DATA_CACHE = "archivist-data-v1";
const CORE_SHELL_URLS = [
  "/",
  "/manifest.webmanifest",
  "/icons/app-icon.svg",
  "/icons/app-icon-maskable.svg",
];
const STATIC_DESTINATIONS = new Set(["script", "style", "font", "image"]);

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(SHELL_CACHE).then((cache) => cache.addAll(CORE_SHELL_URLS)),
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => ![SHELL_CACHE, ASSET_CACHE, DATA_CACHE].includes(key))
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  if (request.destination === "image") {
    event.respondWith(staleWhileRevalidate(request, ASSET_CACHE));
    return;
  }

  if (url.origin !== self.location.origin) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(networkFirst(request, SHELL_CACHE));
    return;
  }

  if (isCatchUpRequest(url.pathname) || isThreadRequest(url.pathname)) {
    event.respondWith(cacheRecentData(request));
    return;
  }

  if (
    STATIC_DESTINATIONS.has(request.destination) ||
    url.pathname === "/manifest.webmanifest"
  ) {
    event.respondWith(staleWhileRevalidate(request, ASSET_CACHE));
  }
});

function isCatchUpRequest(pathname) {
  return pathname === "/api/catch-up";
}

function isThreadRequest(pathname) {
  return /^\/api\/threads\/[^/]+$/.test(pathname);
}

async function networkFirst(request, cacheName) {
  const cache = await caches.open(cacheName);

  try {
    const response = await fetch(request);
    if (response.ok) {
      cache.put(request, response.clone());
    }
    return response;
  } catch {
    return (await cache.match(request)) || (await cache.match("/"));
  }
}

async function staleWhileRevalidate(request, cacheName) {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request);
  const network = fetch(request)
    .then((response) => {
      if (response.ok || response.type === "opaque") {
        cache.put(request, response.clone());
      }
      return response;
    })
    .catch(() => cached);

  return cached || network;
}

async function cacheRecentData(request) {
  const cache = await caches.open(DATA_CACHE);

  try {
    const response = await fetch(request);
    if (response.ok) {
      await cache.put(request, response.clone());
      await trimDataCache(cache);
    }
    return response;
  } catch {
    const cached = await cache.match(request);
    if (cached) {
      return cached;
    }

    throw new Error("No cached data available");
  }
}

async function trimDataCache(cache) {
  const keys = await cache.keys();
  const catchUpKeys = keys.filter((key) => isCatchUpRequest(new URL(key.url).pathname));
  const threadKeys = keys.filter((key) => isThreadRequest(new URL(key.url).pathname));

  await trimCacheEntries(cache, catchUpKeys, 6);
  await trimCacheEntries(cache, threadKeys, 24);
}

async function trimCacheEntries(cache, keys, maxEntries) {
  const excess = keys.length - maxEntries;
  if (excess <= 0) {
    return;
  }

  await Promise.all(
    keys.slice(0, excess).map((key) => cache.delete(key)),
  );
}

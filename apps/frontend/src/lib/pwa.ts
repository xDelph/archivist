type ServiceWorkerRegistrar = Pick<ServiceWorkerContainer, "register">;
type ServiceWorkerDisabler = Pick<ServiceWorkerContainer, "getRegistrations">;
const SHELL_CACHE = "arkivist-shell-v2";
const ASSET_CACHE = "arkivist-assets-v2";
const CORE_SHELL_URLS = [
	"/",
	"/manifest.webmanifest",
	"/icons/app-icon.svg",
	"/icons/app-icon-maskable.svg",
];

export interface ServiceWorkerEnvironment {
	serviceWorker?: ServiceWorkerRegistrar;
	isDevelopment?: boolean;
}

export function shouldRegisterServiceWorker({
	serviceWorker,
	isDevelopment = false,
}: ServiceWorkerEnvironment) {
	return !isDevelopment && typeof serviceWorker !== "undefined";
}

export async function registerAppServiceWorker(
	serviceWorker: ServiceWorkerRegistrar | undefined = globalThis.navigator
		?.serviceWorker,
	isDevelopment = false,
) {
	if (
		!shouldRegisterServiceWorker({
			serviceWorker,
			isDevelopment,
		})
	) {
		return false;
	}

	try {
		await serviceWorker.register("/sw.js", { scope: "/" });
		void warmOfflineShellAssets();
		return true;
	} catch {
		return false;
	}
}

export async function disableAppServiceWorker(
	serviceWorker: ServiceWorkerDisabler | undefined = globalThis.navigator
		?.serviceWorker,
	cacheStorage: CacheStorage | undefined = globalThis.caches,
) {
	if (!serviceWorker) {
		return false;
	}

	try {
		const registrations = await serviceWorker.getRegistrations();
		await Promise.all(
			registrations.map((registration) => registration.unregister()),
		);
		if (cacheStorage) {
			await Promise.all([
				cacheStorage.delete(SHELL_CACHE),
				cacheStorage.delete(ASSET_CACHE),
			]);
		}
		return true;
	} catch {
		return false;
	}
}

export function collectCurrentShellAssetUrls(
	documentRef: Document | undefined = globalThis.document,
	locationRef: Location | undefined = globalThis.location,
) {
	if (!documentRef || !locationRef) {
		return [];
	}

	const urls = new Set<string>();
	for (const element of documentRef.querySelectorAll<
		HTMLLinkElement | HTMLScriptElement
	>(
		'link[href][rel="stylesheet"], link[href][rel="modulepreload"], script[src]',
	)) {
		const candidate = "href" in element ? element.href : element.src;
		if (!candidate) {
			continue;
		}

		const url = new URL(candidate, locationRef.origin);
		if (url.origin === locationRef.origin) {
			urls.add(`${url.pathname}${url.search}`);
		}
	}

	return [...urls];
}

export async function warmOfflineShellAssets(
	cacheStorage: CacheStorage | undefined = globalThis.caches,
	documentRef: Document | undefined = globalThis.document,
	locationRef: Location | undefined = globalThis.location,
	fetchImpl: typeof fetch | undefined = globalThis.fetch,
) {
	if (!cacheStorage || !locationRef || !fetchImpl) {
		return false;
	}

	const shellCache = await cacheStorage.open(SHELL_CACHE);
	const shellUrls = new Set([...CORE_SHELL_URLS, locationRef.pathname || "/"]);
	await Promise.all(
		[...shellUrls].map((url) =>
			cacheUrl(url, locationRef.origin, shellCache, fetchImpl),
		),
	);

	const assetUrls = collectCurrentShellAssetUrls(documentRef, locationRef);
	if (!assetUrls.length) {
		return true;
	}

	const assetCache = await cacheStorage.open(ASSET_CACHE);
	await Promise.all(
		assetUrls.map((url) =>
			cacheUrl(url, locationRef.origin, assetCache, fetchImpl),
		),
	);
	return true;
}

async function cacheUrl(
	url: string,
	origin: string,
	cache: Cache,
	fetchImpl: typeof fetch,
) {
	const request = new Request(new URL(url, origin), {
		credentials: "same-origin",
	});

	try {
		const response = await fetchImpl(request);
		if (response.ok || response.type === "opaque") {
			await cache.put(request, response.clone());
		}
	} catch {
		// Offline shell warming is best-effort and should not block registration.
	}
}

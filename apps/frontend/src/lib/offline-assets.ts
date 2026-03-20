import type { SavedItem, ThreadDetailResponse } from "@/lib/api";

const OFFLINE_ASSET_CACHE = "arkivist-assets-v2";

export function collectOfflineAvatarUrls(
	item: SavedItem,
	thread: ThreadDetailResponse,
) {
	const urls = new Set<string>();

	if (item.author?.avatar_url) {
		urls.add(item.author.avatar_url);
	}

	for (const message of thread.messages) {
		if (message.author?.avatar_url) {
			urls.add(message.author.avatar_url);
		}
	}

	return [...urls];
}

export async function cacheOfflineImage(
	url: string | null | undefined,
	cacheStorage: CacheStorage | undefined = globalThis.caches,
	fetchImpl: typeof fetch | undefined = globalThis.fetch,
) {
	if (!url || !cacheStorage || !fetchImpl) {
		return;
	}

	try {
		const cache = await cacheStorage.open(OFFLINE_ASSET_CACHE);
		const request = new Request(url, {
			mode: "no-cors",
			credentials: "omit",
		});
		const cached = await cache.match(request);

		if (cached) {
			return;
		}

		const response = await fetchImpl(request);
		if (response.ok || response.type === "opaque") {
			await cache.put(request, response.clone());
		}
	} catch {
		// Offline media pinning is best-effort and should never block saved reads.
	}
}

export async function cacheOfflineThreadAvatars(
	item: SavedItem,
	thread: ThreadDetailResponse,
) {
	await Promise.all(
		collectOfflineAvatarUrls(item, thread).map((url) => cacheOfflineImage(url)),
	);
}

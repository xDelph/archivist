import type { SavedItem, ThreadDetailResponse } from "@/lib/api";

export function canReadPathOffline(pathname: string) {
	return (
		pathname === "/saved" ||
		pathname === "/account" ||
		pathname.startsWith("/threads/")
	);
}

export function resolveSavedItemsForReading(
	remoteItems: SavedItem[] | undefined,
	offlineItems: SavedItem[] | undefined,
) {
	if (remoteItems !== undefined) {
		return remoteItems;
	}

	return offlineItems ?? [];
}

export function filterSavedItemsWithSnapshots(
	items: SavedItem[] | undefined,
	offlineThreadIds: string[] | undefined,
) {
	if (!items || !offlineThreadIds) {
		return [];
	}

	const availableThreadIds = new Set(offlineThreadIds);
	return items.filter((item) => availableThreadIds.has(item.thread_id));
}

export function findSavedItemForReading(
	threadId: string,
	remoteItems: SavedItem[] | undefined,
	offlineItems: SavedItem[] | undefined,
) {
	if (remoteItems !== undefined) {
		return remoteItems.find((item) => item.thread_id === threadId) ?? null;
	}

	return offlineItems?.find((item) => item.thread_id === threadId) ?? null;
}

export function resolveThreadForReading(
	remoteThread: ThreadDetailResponse | undefined,
	offlineThread: ThreadDetailResponse | null | undefined,
) {
	return remoteThread ?? offlineThread ?? null;
}

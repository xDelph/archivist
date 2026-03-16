import type { CurrentUserResponse } from "@/lib/api";

const CURRENT_USER_STORAGE_KEY = "archivist.current-user";

export function readCachedCurrentUser(
	storage: Storage | undefined = globalThis.localStorage,
) {
	if (!storage) {
		return null;
	}

	const raw = storage.getItem(CURRENT_USER_STORAGE_KEY);
	if (!raw) {
		return null;
	}

	try {
		return JSON.parse(raw) as CurrentUserResponse;
	} catch {
		return null;
	}
}

export function writeCachedCurrentUser(
	response: CurrentUserResponse,
	storage: Storage | undefined = globalThis.localStorage,
) {
	storage?.setItem(CURRENT_USER_STORAGE_KEY, JSON.stringify(response));
}

export function clearCachedCurrentUser(
	storage: Storage | undefined = globalThis.localStorage,
) {
	storage?.removeItem(CURRENT_USER_STORAGE_KEY);
}

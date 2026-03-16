import { useSyncExternalStore } from "react";

export function useNetworkStatus() {
	const isOnline = useSyncExternalStore(
		subscribeToNetworkChanges,
		getNetworkSnapshot,
		() => true,
	);

	return {
		isOnline,
	};
}

function subscribeToNetworkChanges(onStoreChange: () => void) {
	if (typeof globalThis.window === "undefined") {
		return () => {};
	}

	globalThis.window.addEventListener("online", onStoreChange);
	globalThis.window.addEventListener("offline", onStoreChange);

	return () => {
		globalThis.window.removeEventListener("online", onStoreChange);
		globalThis.window.removeEventListener("offline", onStoreChange);
	};
}

function getNetworkSnapshot() {
	return globalThis.navigator?.onLine !== false;
}

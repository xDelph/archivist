import type { CatchUpWindow } from "@/lib/api";
import { type HomeTab, normalizeHomeTab } from "@/lib/home-tabs";
import {
	type ThreadListSort,
	normalizeThreadListSort,
} from "@/lib/thread-list-sort";

export interface HomeEntryTarget {
	tab: HomeTab;
	channelId?: string;
	sort: ThreadListSort;
	window?: CatchUpWindow;
}

export function resolveHomeEntryTarget(
	pathname: string,
	search: string,
): HomeEntryTarget | null {
	if (pathname !== "/") {
		return null;
	}

	const params = new URLSearchParams(search);
	const tab = normalizeHomeTab(params.get("tab") ?? undefined);
	const channelId = params.get("channel") || undefined;

	if (tab === "starred") {
		return {
			tab,
			channelId,
			sort: "date",
		};
	}

	return {
		tab,
		channelId,
		sort: normalizeThreadListSort(params.get("sort") ?? undefined),
		window: tab === "fresh" ? "24h" : "7d",
	};
}

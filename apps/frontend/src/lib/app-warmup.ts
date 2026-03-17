import {
	type CatchUpSort,
	type CatchUpWindow,
	type SavedItem,
	fetchThreadDetail,
} from "@/lib/api";
import { CATCH_UP_PAGE_SIZE } from "@/lib/catch-up";
import { type HomeTab, normalizeHomeTab } from "@/lib/home-tabs";
import {
	replaceOfflineSavedItems,
	storeOfflineSavedThread,
} from "@/lib/offline-library";
import {
	catchUpQueries,
	highlightQueries,
	offlineQueries,
	savedQueries,
} from "@/lib/queries";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";

const BACKGROUND_WARMUP_DELAY_MS = 700;

interface CatchUpWarmTarget {
	window: CatchUpWindow;
	sort?: CatchUpSort;
	channelId?: string;
}

interface HighlightWarmTarget {
	channelId?: string;
}

type HomeWarmTarget =
	| ({ tab: "highlights" } & HighlightWarmTarget)
	| ({ tab: Exclude<HomeTab, "highlights"> } & CatchUpWarmTarget);

export function buildHomeWarmTargets(pathname: string, search: string) {
	const params = new URLSearchParams(search);
	const channelId = params.get("channel") || undefined;
	const activeTab =
		pathname === "/" ? normalizeHomeTab(params.get("tab")) : null;
	const targets: HomeWarmTarget[] = [
		{ tab: "highlights", channelId },
		{ tab: "fresh", window: "24h", channelId },
		{ tab: "steady", window: "7d", channelId },
		{ tab: "trending", window: "7d", sort: "trending", channelId },
	];

	return targets.filter(
		(target) => pathname !== "/" || target.tab !== activeTab,
	);
}

export function getHomeWarmTargetKey(target: HomeWarmTarget) {
	if (target.tab === "highlights") {
		return `highlights:${target.channelId ?? "all"}`;
	}

	return `${target.window}:${target.sort ?? "activity"}:${target.channelId ?? "all"}`;
}

export function useAppWarmup({
	isOnline,
	pathname,
	search,
}: {
	isOnline: boolean;
	pathname: string;
	search: string;
}) {
	const queryClient = useQueryClient();
	const savedQuery = useQuery({
		...savedQueries.list(),
		enabled: isOnline,
	});
	const warmedCatchUpTargetsRef = useRef(new Set<string>());
	const syncingSavedThreadsRef = useRef(new Set<string>());

	useEffect(() => {
		if (!isOnline || typeof window === "undefined") {
			return;
		}

		const targets = buildHomeWarmTargets(pathname, search).filter(
			(target) =>
				!warmedCatchUpTargetsRef.current.has(getHomeWarmTargetKey(target)),
		);
		if (!targets.length) {
			return;
		}

		const timeout = window.setTimeout(() => {
			for (const target of targets) {
				warmedCatchUpTargetsRef.current.add(getHomeWarmTargetKey(target));
				if (target.tab === "highlights") {
					void queryClient.prefetchQuery(
						highlightQueries.list({ channelId: target.channelId }),
					);
					continue;
				}

				void queryClient.prefetchInfiniteQuery(
					catchUpQueries.feed({
						window: target.window,
						channelId: target.channelId,
						sort: target.sort,
						limit: CATCH_UP_PAGE_SIZE,
					}),
				);
			}
		}, BACKGROUND_WARMUP_DELAY_MS);

		return () => window.clearTimeout(timeout);
	}, [isOnline, pathname, queryClient, search]);

	useEffect(() => {
		if (!isOnline || !savedQuery.data || typeof window === "undefined") {
			return;
		}

		const timeout = window.setTimeout(() => {
			void warmSavedThreadsForOffline(
				savedQuery.data.items,
				queryClient,
				syncingSavedThreadsRef.current,
			);
		}, BACKGROUND_WARMUP_DELAY_MS);

		return () => window.clearTimeout(timeout);
	}, [isOnline, queryClient, savedQuery.data]);
}

async function warmSavedThreadsForOffline(
	items: SavedItem[],
	queryClient: ReturnType<typeof useQueryClient>,
	syncingThreadIds: Set<string>,
) {
	await replaceOfflineSavedItems(items);
	await queryClient.invalidateQueries({ queryKey: ["offline", "saved"] });
	await queryClient.invalidateQueries({ queryKey: ["offline", "thread-ids"] });

	const offlineThreadIds = new Set(
		await queryClient.fetchQuery(offlineQueries.threadIds()),
	);

	for (const item of items) {
		if (
			offlineThreadIds.has(item.thread_id) ||
			syncingThreadIds.has(item.thread_id)
		) {
			continue;
		}

		syncingThreadIds.add(item.thread_id);
		void fetchThreadDetail(item.thread_id)
			.then((thread) => storeOfflineSavedThread(item, thread))
			.then(async () => {
				await queryClient.invalidateQueries({
					queryKey: ["offline", "thread", item.thread_id],
				});
				await queryClient.invalidateQueries({
					queryKey: ["offline", "thread-ids"],
				});
			})
			.finally(() => {
				syncingThreadIds.delete(item.thread_id);
			});
	}
}

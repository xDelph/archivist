import {
	type CatchUpSort,
	type CatchUpWindow,
	type ChannelSummary,
	type SearchParams,
	fetchCatchUp,
	fetchChannels,
	fetchCurrentUser,
	fetchSavedItems,
	fetchSearchResults,
	fetchStarredItems,
	fetchThreadDetail,
	isApiErrorWithStatus,
} from "@/lib/api";
import {
	clearCachedCurrentUser,
	readCachedCurrentUser,
	writeCachedCurrentUser,
} from "@/lib/auth-cache";
import { CATCH_UP_PAGE_SIZE } from "@/lib/catch-up";
import { cacheOfflineImage } from "@/lib/offline-assets";
import {
	getOfflineThreadDetail,
	listOfflineSavedItems,
	listOfflineThreadDetailIds,
} from "@/lib/offline-library";
import { SEARCH_PAGE_SIZE } from "@/lib/search";
import { infiniteQueryOptions, queryOptions } from "@tanstack/react-query";
import { z } from "zod";

export const searchSearchSchema = z.object({
	q: z.string().optional().catch(undefined),
	channel_id: z.string().optional().catch(undefined),
	date_from: z.string().optional().catch(undefined),
	date_to: z.string().optional().catch(undefined),
	sort: z.enum(["relevance", "newest"]).optional().catch(undefined),
});

export type SearchSearchParams = z.infer<typeof searchSearchSchema>;

export const authQueries = {
	me: () =>
		queryOptions({
			queryKey: ["auth", "me"],
			queryFn: async () => {
				try {
					const response = await fetchCurrentUser();
					writeCachedCurrentUser(response);
					void cacheOfflineImage(response.user.avatar_url);
					return response;
				} catch (error) {
					if (isApiErrorWithStatus(error, 401)) {
						clearCachedCurrentUser();
						throw error;
					}

					const cached = readCachedCurrentUser();
					if (cached) {
						return cached;
					}

					throw error;
				}
			},
			staleTime: 60_000,
			retry: false,
		}),
};

export const catchUpQueries = {
	summary: (window: CatchUpWindow) =>
		queryOptions({
			queryKey: ["catch-up", window, "summary"],
			queryFn: () => fetchCatchUp({ window, limit: 1 }),
			staleTime: 30_000,
		}),
	feedKey: ({
		window,
		channelId,
		sort = "activity",
	}: {
		window: CatchUpWindow;
		channelId?: string;
		sort?: CatchUpSort;
	}) => ["catch-up", window, channelId ?? "all", sort] as const,
	feed: ({
		window,
		channelId,
		sort = "activity",
		limit = CATCH_UP_PAGE_SIZE,
	}: {
		window: CatchUpWindow;
		channelId?: string;
		sort?: CatchUpSort;
		limit?: number;
	}) =>
		infiniteQueryOptions({
			queryKey: catchUpQueries.feedKey({
				window,
				channelId,
				sort,
			}),
			queryFn: ({ pageParam }) =>
				fetchCatchUp({
					window,
					channelId,
					sort,
					cursor: pageParam || undefined,
					limit,
				}),
			initialPageParam: "",
			getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
			staleTime: 30_000,
		}),
};

export const channelQueries = {
	list: () =>
		queryOptions({
			queryKey: ["channels"],
			queryFn: fetchChannels,
			select: (channels: ChannelSummary[]) =>
				channels
					.filter((channel) => channel.name)
					.sort((left, right) =>
						(left.name ?? left.id).localeCompare(right.name ?? right.id),
					),
			staleTime: 60_000,
		}),
};

export const searchQueries = {
	results: (params: SearchParams) =>
		infiniteQueryOptions({
			queryKey: ["search", params],
			queryFn: ({ pageParam }) =>
				fetchSearchResults({
					...params,
					cursor: pageParam || undefined,
					limit: SEARCH_PAGE_SIZE,
				}),
			initialPageParam: "",
			getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
			enabled: params.query.length > 0,
		}),
};

export const threadQueries = {
	detail: (threadId: string) =>
		queryOptions({
			queryKey: ["thread", threadId],
			queryFn: () => fetchThreadDetail(threadId),
		}),
};

export const savedQueries = {
	list: () =>
		queryOptions({
			queryKey: ["saved"],
			queryFn: fetchSavedItems,
			staleTime: 30_000,
		}),
};

export const starredQueries = {
	list: ({ channelId }: { channelId?: string } = {}) =>
		queryOptions({
			queryKey: ["starred", channelId ?? "all"],
			queryFn: () => fetchStarredItems(channelId),
			staleTime: 30_000,
		}),
};

export const offlineQueries = {
	saved: () =>
		queryOptions({
			queryKey: ["offline", "saved"],
			queryFn: listOfflineSavedItems,
			staleTime: Number.POSITIVE_INFINITY,
		}),
	threadIds: () =>
		queryOptions({
			queryKey: ["offline", "thread-ids"],
			queryFn: listOfflineThreadDetailIds,
			staleTime: Number.POSITIVE_INFINITY,
		}),
	thread: (threadId: string) =>
		queryOptions({
			queryKey: ["offline", "thread", threadId],
			queryFn: () => getOfflineThreadDetail(threadId),
			staleTime: Number.POSITIVE_INFINITY,
		}),
};

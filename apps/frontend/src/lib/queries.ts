import {
	type ChannelSummary,
	type SearchParams,
	fetchCatchUp,
	fetchChannels,
	fetchCurrentUser,
	fetchSavedItems,
	fetchSearchResults,
	fetchThreadDetail,
} from "@/lib/api";
import { queryOptions } from "@tanstack/react-query";
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
			queryFn: fetchCurrentUser,
			staleTime: 60_000,
			retry: false,
		}),
};

export const catchUpQueries = {
	window: (window: "24h" | "7d") =>
		queryOptions({
			queryKey: ["catch-up", window],
			queryFn: () => fetchCatchUp(window),
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
		queryOptions({
			queryKey: ["search", params],
			queryFn: () => fetchSearchResults(params),
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

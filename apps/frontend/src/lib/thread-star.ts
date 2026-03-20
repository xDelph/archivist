import type { StarredItem } from "@/lib/api";
import { starThread, unstarThread } from "@/lib/api";
import {
	type QueryClient,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";

interface StarredItemsResponseShape {
	items: StarredItem[];
}

export function indexStarredItemsByThreadId(items?: StarredItem[]) {
	return new Map((items ?? []).map((item) => [item.thread_id, item] as const));
}

export function upsertStarredItem(
	items: StarredItem[] | undefined,
	item: StarredItem,
) {
	const nextItems = (items ?? []).filter(
		(candidate) => candidate.thread_id !== item.thread_id,
	);
	nextItems.unshift(item);
	return nextItems;
}

export function removeStarredItemByThreadId(
	items: StarredItem[] | undefined,
	threadId: string,
) {
	return (items ?? []).filter((item) => item.thread_id !== threadId);
}

export function useThreadStarAction({
	threadId,
	starredItem,
}: {
	threadId: string;
	starredItem?: StarredItem | null;
}) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async () => {
			if (starredItem) {
				await unstarThread(threadId);
				return { kind: "removed" } as const;
			}

			const result = await starThread(threadId);
			return { kind: "starred", item: result.item } as const;
		},
		onSuccess: async (result) => {
			if (result.kind === "removed") {
				removeStarredItemFromCaches(queryClient, threadId);
			} else {
				upsertStarredItemInCaches(queryClient, result.item);
			}
			await queryClient.invalidateQueries({ queryKey: ["starred"] });
		},
	});
}

function upsertStarredItemInCaches(
	queryClient: QueryClient,
	item: StarredItem,
) {
	for (const [queryKey, current] of queryClient.getQueriesData<
		StarredItemsResponseShape | undefined
	>({
		queryKey: ["starred"],
	})) {
		const channelKey = getStarredQueryChannelKey(queryKey);
		if (channelKey !== "all" && channelKey !== item.channel_id) {
			continue;
		}

		queryClient.setQueryData<StarredItemsResponseShape | undefined>(queryKey, {
			items: upsertStarredItem(current?.items, item),
		});
	}
}

function removeStarredItemFromCaches(
	queryClient: QueryClient,
	threadId: string,
) {
	for (const [queryKey, current] of queryClient.getQueriesData<
		StarredItemsResponseShape | undefined
	>({
		queryKey: ["starred"],
	})) {
		queryClient.setQueryData<StarredItemsResponseShape | undefined>(queryKey, {
			items: removeStarredItemByThreadId(current?.items, threadId),
		});
	}
}

function getStarredQueryChannelKey(queryKey: readonly unknown[]) {
	return typeof queryKey[1] === "string" ? queryKey[1] : "all";
}

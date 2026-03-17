import type { HighlightedItem } from "@/lib/api";
import { deleteHighlightedThread, pinHighlightedThread } from "@/lib/api";
import {
	type QueryClient,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";

interface HighlightItemsResponseShape {
	items: HighlightedItem[];
}

export function indexHighlightedItemsByThreadId(items?: HighlightedItem[]) {
	return new Map((items ?? []).map((item) => [item.thread_id, item] as const));
}

export function upsertHighlightedItem(
	items: HighlightedItem[] | undefined,
	item: HighlightedItem,
) {
	const nextItems = (items ?? []).filter(
		(candidate) => candidate.thread_id !== item.thread_id,
	);
	nextItems.unshift(item);
	return nextItems;
}

export function removeHighlightedItemByThreadId(
	items: HighlightedItem[] | undefined,
	threadId: string,
) {
	return (items ?? []).filter((item) => item.thread_id !== threadId);
}

export function useThreadHighlightAction({
	threadId,
	highlightedItem,
}: {
	threadId: string;
	highlightedItem?: HighlightedItem | null;
}) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async () => {
			if (highlightedItem) {
				await deleteHighlightedThread(threadId);
				return { kind: "removed" } as const;
			}

			const result = await pinHighlightedThread(threadId);
			return { kind: "highlighted", item: result.item } as const;
		},
		onSuccess: async (result) => {
			if (result.kind === "removed") {
				removeHighlightedItemFromCaches(queryClient, threadId);
			} else {
				upsertHighlightedItemInCaches(queryClient, result.item);
			}
			await queryClient.invalidateQueries({ queryKey: ["highlights"] });
		},
	});
}

function upsertHighlightedItemInCaches(
	queryClient: QueryClient,
	item: HighlightedItem,
) {
	for (const [queryKey, current] of queryClient.getQueriesData<
		HighlightItemsResponseShape | undefined
	>({
		queryKey: ["highlights"],
	})) {
		const channelKey = getHighlightQueryChannelKey(queryKey);
		if (channelKey !== "all" && channelKey !== item.channel_id) {
			continue;
		}

		queryClient.setQueryData<HighlightItemsResponseShape | undefined>(
			queryKey,
			{
				items: upsertHighlightedItem(current?.items, item),
			},
		);
	}
}

function removeHighlightedItemFromCaches(
	queryClient: QueryClient,
	threadId: string,
) {
	for (const [queryKey, current] of queryClient.getQueriesData<
		HighlightItemsResponseShape | undefined
	>({
		queryKey: ["highlights"],
	})) {
		queryClient.setQueryData<HighlightItemsResponseShape | undefined>(
			queryKey,
			{
				items: removeHighlightedItemByThreadId(current?.items, threadId),
			},
		);
	}
}

function getHighlightQueryChannelKey(queryKey: readonly unknown[]) {
	return typeof queryKey[1] === "string" ? queryKey[1] : "all";
}

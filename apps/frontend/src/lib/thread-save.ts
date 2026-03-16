import {
	type SavedItem,
	type ThreadDetailResponse,
	deleteSavedThread,
	saveThread,
} from "@/lib/api";
import {
	removeOfflineSavedThread,
	storeOfflineSavedThread,
} from "@/lib/offline-library";
import { savedQueries, threadQueries } from "@/lib/queries";
import {
	type QueryClient,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";

interface SavedItemsResponseShape {
	items: SavedItem[];
}

export function indexSavedItemsByThreadId(items?: SavedItem[]) {
	return new Map((items ?? []).map((item) => [item.thread_id, item] as const));
}

export function upsertSavedItem(
	items: SavedItem[] | undefined,
	item: SavedItem,
) {
	const nextItems = (items ?? []).filter(
		(candidate) => candidate.thread_id !== item.thread_id,
	);
	nextItems.unshift(item);
	return nextItems;
}

export function removeSavedItemByThreadId(
	items: SavedItem[] | undefined,
	threadId: string,
) {
	return (items ?? []).filter((item) => item.thread_id !== threadId);
}

export function useThreadSaveAction({
	threadId,
	savedItem,
	threadDetail,
}: {
	threadId: string;
	savedItem?: SavedItem | null;
	threadDetail?: ThreadDetailResponse | null;
}) {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async () => {
			if (savedItem) {
				await deleteSavedThread(threadId);
				return { kind: "removed" } as const;
			}

			const result = await saveThread(threadId);
			return { kind: "saved", item: result.item } as const;
		},
		onSuccess: async (result) => {
			if (result.kind === "removed") {
				updateSavedItemsCache(queryClient, (items) =>
					removeSavedItemByThreadId(items, threadId),
				);
				await removeOfflineSavedThread(threadId);
			} else {
				updateSavedItemsCache(queryClient, (items) =>
					upsertSavedItem(items, result.item),
				);
				const detail =
					threadDetail ??
					(await fetchThreadDetailForOffline(queryClient, threadId));
				if (detail) {
					await storeOfflineSavedThread(result.item, detail);
				}
			}

			await queryClient.invalidateQueries({ queryKey: ["saved"] });
			await queryClient.invalidateQueries({ queryKey: ["offline"] });
		},
	});
}

async function fetchThreadDetailForOffline(
	queryClient: QueryClient,
	threadId: string,
) {
	try {
		return await queryClient.fetchQuery(threadQueries.detail(threadId));
	} catch {
		return null;
	}
}

function updateSavedItemsCache(
	queryClient: QueryClient,
	updater: (items: SavedItem[] | undefined) => SavedItem[],
) {
	queryClient.setQueryData<SavedItemsResponseShape | undefined>(
		savedQueries.list().queryKey,
		(current) => ({
			items: updater(current?.items),
		}),
	);
}

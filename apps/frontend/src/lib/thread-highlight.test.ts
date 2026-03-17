import type { HighlightedItem } from "@/lib/api";
import {
	indexHighlightedItemsByThreadId,
	removeHighlightedItemByThreadId,
	upsertHighlightedItem,
} from "@/lib/thread-highlight";

const firstItem: HighlightedItem = {
	id: "highlight-1",
	thread_id: "C123:1",
	channel_id: "C123",
	channel_name: "news",
	author: null,
	root_ts: "1",
	title: "First highlight",
	preview: "Preview",
	reply_count: 3,
	participant_count: 2,
	reaction_count: 1,
	file_count: 0,
	last_activity_ts: "1710000000.000001",
	pinned_at: "2026-03-17T10:00:00Z",
};

const secondItem: HighlightedItem = {
	...firstItem,
	id: "highlight-2",
	thread_id: "C123:2",
	root_ts: "2",
	title: "Second highlight",
};

describe("thread highlight helpers", () => {
	it("indexes highlighted items by thread id", () => {
		const index = indexHighlightedItemsByThreadId([firstItem, secondItem]);

		expect(index.get(firstItem.thread_id)).toEqual(firstItem);
		expect(index.get(secondItem.thread_id)).toEqual(secondItem);
	});

	it("upserts highlighted items to the front of the list", () => {
		expect(upsertHighlightedItem([firstItem], secondItem)).toEqual([
			secondItem,
			firstItem,
		]);
		expect(
			upsertHighlightedItem([firstItem], {
				...firstItem,
				title: "Updated highlight",
			}),
		).toEqual([{ ...firstItem, title: "Updated highlight" }]);
	});

	it("removes highlighted items by thread id", () => {
		expect(
			removeHighlightedItemByThreadId(
				[firstItem, secondItem],
				firstItem.thread_id,
			),
		).toEqual([secondItem]);
	});
});

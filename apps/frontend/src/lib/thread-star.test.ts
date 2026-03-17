import type { StarredItem } from "@/lib/api";
import {
	indexStarredItemsByThreadId,
	removeStarredItemByThreadId,
	upsertStarredItem,
} from "@/lib/thread-star";

const firstItem: StarredItem = {
	id: "star-1",
	thread_id: "C123:1",
	channel_id: "C123",
	channel_name: "news",
	author: null,
	root_ts: "1",
	title: "First star",
	preview: "Preview",
	reply_count: 3,
	participant_count: 2,
	reaction_count: 1,
	file_count: 0,
	last_activity_ts: "1710000000.000001",
	pinned_at: "2026-03-17T10:00:00Z",
};

const secondItem: StarredItem = {
	...firstItem,
	id: "star-2",
	thread_id: "C123:2",
	root_ts: "2",
	title: "Second star",
};

describe("thread star helpers", () => {
	it("indexes starred items by thread id", () => {
		const index = indexStarredItemsByThreadId([firstItem, secondItem]);

		expect(index.get(firstItem.thread_id)).toEqual(firstItem);
		expect(index.get(secondItem.thread_id)).toEqual(secondItem);
	});

	it("upserts starred items to the front of the list", () => {
		expect(upsertStarredItem([firstItem], secondItem)).toEqual([
			secondItem,
			firstItem,
		]);
		expect(
			upsertStarredItem([firstItem], {
				...firstItem,
				title: "Updated star",
			}),
		).toEqual([{ ...firstItem, title: "Updated star" }]);
	});

	it("removes starred items by thread id", () => {
		expect(
			removeStarredItemByThreadId(
				[firstItem, secondItem],
				firstItem.thread_id,
			),
		).toEqual([secondItem]);
	});
});

import type { SavedItem } from "@/lib/api";
import {
	indexSavedItemsByThreadId,
	removeSavedItemByThreadId,
	upsertSavedItem,
} from "@/lib/thread-save";

const firstItem: SavedItem = {
	id: "saved-1",
	thread_id: "C123:1",
	channel_id: "C123",
	channel_name: "news",
	author: null,
	root_ts: "1",
	title: "First thread",
	preview: "Preview",
	summary_preview: null,
	preview_source: "fallback",
	reply_count: 3,
	participant_count: 2,
	reaction_count: 1,
	file_count: 0,
	last_activity_ts: "1710000000.000001",
	saved_at: "2026-03-16T10:00:00Z",
};

const secondItem: SavedItem = {
	...firstItem,
	id: "saved-2",
	thread_id: "C123:2",
	root_ts: "2",
	title: "Second thread",
};

describe("thread save helpers", () => {
	it("indexes saved items by thread id", () => {
		const index = indexSavedItemsByThreadId([firstItem, secondItem]);

		expect(index.get(firstItem.thread_id)).toEqual(firstItem);
		expect(index.get(secondItem.thread_id)).toEqual(secondItem);
	});

	it("upserts saved items to the front of the list", () => {
		expect(upsertSavedItem([firstItem], secondItem)).toEqual([
			secondItem,
			firstItem,
		]);
		expect(
			upsertSavedItem([firstItem], { ...firstItem, title: "Updated" }),
		).toEqual([{ ...firstItem, title: "Updated" }]);
	});

	it("removes saved items by thread id", () => {
		expect(
			removeSavedItemByThreadId([firstItem, secondItem], firstItem.thread_id),
		).toEqual([secondItem]);
	});
});

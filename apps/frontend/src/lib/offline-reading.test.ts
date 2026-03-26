import type { SavedItem, ThreadDetailResponse } from "@/lib/api";
import { describe, expect, it } from "vitest";

import {
	canReadPathOffline,
	filterSavedItemsWithSnapshots,
	findSavedItemForReading,
	resolveSavedItemsForReading,
	resolveThreadForReading,
} from "./offline-reading";

const savedItem: SavedItem = {
	id: "saved-1",
	thread_id: "C123:1700000000.000001",
	channel_id: "C123",
	channel_name: "general",
	author: null,
	root_ts: "1700000000.000001",
	title: "Saved thread",
	preview: "Preview",
	summary_preview: null,
	preview_source: "fallback",
	reply_count: 3,
	participant_count: 2,
	reaction_count: 1,
	file_count: 0,
	last_activity_ts: "1700000000.000002",
	saved_at: "2026-03-16T10:00:00.000Z",
};

const offlineThread: ThreadDetailResponse = {
	id: savedItem.thread_id,
	channel_id: savedItem.channel_id,
	channel_name: savedItem.channel_name,
	root_ts: savedItem.root_ts,
	title: savedItem.title,
	preview: savedItem.preview,
	last_activity_ts: savedItem.last_activity_ts,
	reply_count: savedItem.reply_count,
	participant_count: savedItem.participant_count,
	reaction_count: savedItem.reaction_count,
	file_count: savedItem.file_count,
	summary: {
		text: "Summary",
		full_summary: "Summary",
		why_it_mattered: null,
		status: null,
		topic_tags: [],
		model: null,
		generated_at: null,
		is_stale: false,
		source: "fallback",
	},
	messages: [],
};

describe("offline reading helpers", () => {
	it("limits offline access to saved, account, and thread routes", () => {
		expect(canReadPathOffline("/saved")).toBe(true);
		expect(canReadPathOffline("/account")).toBe(true);
		expect(canReadPathOffline("/threads/C123:1")).toBe(true);
		expect(canReadPathOffline("/")).toBe(false);
		expect(canReadPathOffline("/search")).toBe(false);
	});

	it("falls back to offline saved items when remote data is unavailable", () => {
		expect(resolveSavedItemsForReading([savedItem], [])).toEqual([savedItem]);
		expect(resolveSavedItemsForReading(undefined, [savedItem])).toEqual([
			savedItem,
		]);
		expect(resolveSavedItemsForReading([], [savedItem])).toEqual([]);
		expect(
			findSavedItemForReading(savedItem.thread_id, undefined, [savedItem]),
		).toEqual(savedItem);
		expect(
			findSavedItemForReading(savedItem.thread_id, [], [savedItem]),
		).toBeNull();
	});

	it("keeps only saved items with downloaded thread snapshots offline", () => {
		expect(
			filterSavedItemsWithSnapshots([savedItem], [savedItem.thread_id]),
		).toEqual([savedItem]);
		expect(filterSavedItemsWithSnapshots([savedItem], [])).toEqual([]);
		expect(filterSavedItemsWithSnapshots([savedItem], undefined)).toEqual([]);
	});

	it("prefers live thread data and falls back to offline snapshots", () => {
		expect(resolveThreadForReading(offlineThread, null)).toBe(offlineThread);
		expect(resolveThreadForReading(undefined, offlineThread)).toBe(
			offlineThread,
		);
		expect(resolveThreadForReading(undefined, null)).toBeNull();
	});
});

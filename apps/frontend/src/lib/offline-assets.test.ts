import type { SavedItem, ThreadDetailResponse } from "@/lib/api";
import { describe, expect, it, vi } from "vitest";

import { cacheOfflineImage, collectOfflineAvatarUrls } from "./offline-assets";

const savedItem: SavedItem = {
	id: "saved-1",
	thread_id: "C123:1700000000.000001",
	channel_id: "C123",
	channel_name: "general",
	author: {
		slack_user_id: "U1",
		display_name: "Alice",
		avatar_url: "https://cdn.example.com/alice.png",
	},
	root_ts: "1700000000.000001",
	title: "Saved thread",
	preview: "Preview",
	reply_count: 3,
	participant_count: 2,
	reaction_count: 1,
	file_count: 0,
	last_activity_ts: "1700000000.000002",
	saved_at: "2026-03-16T10:00:00.000Z",
};

const thread: ThreadDetailResponse = {
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
		why_it_mattered: null,
		status: null,
		topic_tags: [],
		model: null,
		generated_at: null,
		is_stale: false,
		source: "fallback",
	},
	messages: [
		{
			ts: "1700000000.000001",
			thread_ts: "1700000000.000001",
			user_id: "U1",
			author: savedItem.author,
			text: "First",
			reactions: [],
			files: [],
		},
		{
			ts: "1700000000.000002",
			thread_ts: "1700000000.000001",
			user_id: "U2",
			author: {
				slack_user_id: "U2",
				display_name: "Bob",
				avatar_url: "https://cdn.example.com/bob.png",
			},
			text: "Second",
			reactions: [],
			files: [],
		},
	],
};

describe("offline assets", () => {
	it("collects unique avatar urls from saved items and thread messages", () => {
		expect(collectOfflineAvatarUrls(savedItem, thread)).toEqual([
			"https://cdn.example.com/alice.png",
			"https://cdn.example.com/bob.png",
		]);
	});

	it("pins opaque cross-origin images into cache storage", async () => {
		const match = vi.fn().mockResolvedValue(undefined);
		const put = vi.fn().mockResolvedValue(undefined);
		const open = vi.fn().mockResolvedValue({
			match,
			put,
		});
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: false,
			type: "opaque",
			clone: () => ({ ok: false, type: "opaque" }),
		});

		await cacheOfflineImage(
			"https://cdn.example.com/alice.png",
			{ open } as unknown as CacheStorage,
			fetchImpl as typeof fetch,
		);

		expect(open).toHaveBeenCalledWith("arkivist-assets-v2");
		expect(match).toHaveBeenCalledOnce();
		expect(fetchImpl).toHaveBeenCalledOnce();
		expect(put).toHaveBeenCalledOnce();
	});
});

import type { CatchUpChannel, CatchUpResponse } from "@/lib/api";
import { describe, expect, it } from "vitest";

import {
	flattenCatchUpPages,
	getCatchUpChannelLabel,
	getCatchUpThreadChannelName,
	pickChannelHighlights,
} from "./catch-up";

describe("flattenCatchUpPages", () => {
	it("preserves paged catch-up items in order", () => {
		const pages: Pick<CatchUpResponse, "items">[] = [
			{
				items: [
					{
						id: "C123:1700000000.000002",
						channel_id: "C123",
						channel_name: "general",
						root_ts: "1700000000.000002",
						author: null,
						title: "Newest thread in first page",
						preview: "newest",
						reply_count: 0,
						participant_count: 1,
						reaction_count: 0,
						file_count: 0,
						last_activity_ts: "1700000000.000003",
					},
				],
			},
			{
				items: [
					{
						id: "C999:1700000000.000004",
						channel_id: "C999",
						channel_name: null,
						root_ts: "1700000000.000004",
						author: null,
						title: "Older thread in second page",
						preview: "older",
						reply_count: 0,
						participant_count: 1,
						reaction_count: 0,
						file_count: 0,
						last_activity_ts: "1700000000.000001",
					},
				],
			},
		];

		expect(flattenCatchUpPages(pages).map((thread) => thread.id)).toEqual([
			"C123:1700000000.000002",
			"C999:1700000000.000004",
		]);
	});
});

describe("catch-up labels", () => {
	it("falls back to ids when names are missing", () => {
		expect(getCatchUpChannelLabel({ id: "C123", name: "general" })).toBe(
			"general",
		);
		expect(getCatchUpChannelLabel({ id: "C123", name: null })).toBe("C123");
		expect(
			getCatchUpThreadChannelName({
				id: "C123:1700000000.000001",
				channel_id: "C123",
				channel_name: null,
				root_ts: "1700000000.000001",
				author: null,
				title: "Thread",
				preview: "Thread",
				reply_count: 0,
				participant_count: 1,
				reaction_count: 0,
				file_count: 0,
				last_activity_ts: "1700000000.000001",
			}),
		).toBe("C123");
	});
});

describe("pickChannelHighlights", () => {
	it("keeps the busiest channels first", () => {
		const channels: CatchUpChannel[] = [
			{
				id: "C123",
				name: "general",
				kind: "public",
				is_archived: false,
				thread_count: 2,
			},
			{
				id: "C999",
				name: "product",
				kind: "public",
				is_archived: false,
				thread_count: 5,
			},
			{
				id: "C456",
				name: "ops",
				kind: "public",
				is_archived: false,
				thread_count: 3,
			},
		];

		expect(
			pickChannelHighlights(channels).map((channel) => channel.id),
		).toEqual(["C999", "C456", "C123"]);
	});
});

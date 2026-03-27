import type { SearchResponse } from "@/lib/api";
import { describe, expect, it } from "vitest";

import { searchQueries } from "./queries";
import { flattenSearchPages } from "./search";

describe("flattenSearchPages", () => {
	it("preserves paged search items in order", () => {
		const pages: Pick<SearchResponse, "items">[] = [
			{
				items: [
					{
						id: "search-1",
						thread_id: "C123:1700000000.000001",
						channel_id: "C123",
						channel_name: "general",
						author: null,
						root_ts: "1700000000.000001",
						message_ts: "1700000000.000002",
						title: "First page result",
						preview: "First page result preview",
						snippet: "first",
						summary_preview: null,
						preview_source: "fallback",
						last_activity_ts: "1700000000.000002",
						reply_count: 2,
						participant_count: 3,
						reaction_count: 1,
						file_count: 0,
						score: 42,
					},
				],
			},
			{
				items: [
					{
						id: "search-2",
						thread_id: "C999:1700000000.000010",
						channel_id: "C999",
						channel_name: "product",
						author: null,
						root_ts: "1700000000.000010",
						message_ts: "1700000000.000011",
						title: "Second page result",
						preview: "Second page result preview",
						snippet: "second",
						summary_preview: null,
						preview_source: "fallback",
						last_activity_ts: "1700000000.000011",
						reply_count: 1,
						participant_count: 2,
						reaction_count: 0,
						file_count: 0,
						score: 21,
					},
				],
			},
		];

		expect(flattenSearchPages(pages).map((item) => item.id)).toEqual([
			"search-1",
			"search-2",
		]);
	});
});

describe("searchQueries.results", () => {
	it("keeps the API cursor for infinite loading", () => {
		const query = searchQueries.results({ query: "archive" });
		const getNextPageParam = query.getNextPageParam;
		expect(getNextPageParam).toBeTypeOf("function");

		expect(
			getNextPageParam?.(
				{ query: "archive", items: [], next_cursor: "20" },
				[],
				"",
				[],
			),
		).toBe("20");
		expect(
			getNextPageParam?.(
				{ query: "archive", items: [], next_cursor: null },
				[],
				"",
				[],
			),
		).toBeUndefined();
	});
});

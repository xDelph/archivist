import type { CatchUpChannel } from "@/lib/api";
import { describe, expect, it } from "vitest";

import { flattenCatchUpThreads } from "./catch-up";

describe("flattenCatchUpThreads", () => {
	it("sorts threads globally by last activity", () => {
		const channels: CatchUpChannel[] = [
			{
				id: "C123",
				name: "general",
				kind: "public",
				is_archived: false,
				thread_count: 2,
				threads: [
					{
						id: "C123:1700000000.000001",
						root_ts: "1700000000.000001",
						author: null,
						title: "Older thread in first channel",
						preview: "older",
						reply_count: 0,
						participant_count: 1,
						reaction_count: 0,
						file_count: 0,
						last_activity_ts: "1700000000.000001",
					},
					{
						id: "C123:1700000000.000002",
						root_ts: "1700000000.000002",
						author: null,
						title: "Newest thread in first channel",
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
				id: "C999",
				name: "product",
				kind: "public",
				is_archived: false,
				thread_count: 1,
				threads: [
					{
						id: "C999:1700000000.000004",
						root_ts: "1700000000.000004",
						author: null,
						title: "Middle thread in second channel",
						preview: "middle",
						reply_count: 0,
						participant_count: 1,
						reaction_count: 0,
						file_count: 0,
						last_activity_ts: "1700000000.000002",
					},
				],
			},
		];

		expect(flattenCatchUpThreads(channels).map((thread) => thread.id)).toEqual([
			"C123:1700000000.000002",
			"C999:1700000000.000004",
			"C123:1700000000.000001",
		]);
	});
});

import {
	threadCardDataFromCatchUpThread,
	threadCardDataFromSavedItem,
	threadCardDataFromSearchResult,
	threadCardDataFromStarredItem,
} from "@/lib/thread-card-props";

describe("thread card props", () => {
	it("prefers the ai summary preview for catch-up cards", () => {
		const card = threadCardDataFromCatchUpThread({
			id: "C123:1700000000.000001",
			channel_id: "C123",
			channel_name: "general",
			root_ts: "1700000000.000001",
			author: null,
			title: "Root title",
			preview: "Root preview",
			summary_preview: "AI preview",
			preview_source: "ai",
			reply_count: 3,
			participant_count: 2,
			reaction_count: 1,
			file_count: 0,
			last_activity_ts: "1700000001.000001",
		});

		expect(card.preview).toBe("AI preview");
		expect(card.previewSource).toBe("ai");
	});

	it("falls back to the thread first message when no ai preview exists", () => {
		const card = threadCardDataFromSearchResult(
			{
				id: "C123:1700000000.000001",
				thread_id: "C123:1700000000.000001",
				channel_id: "C123",
				channel_name: "general",
				author: null,
				root_ts: "1700000000.000001",
				message_ts: "1700000001.000001",
				title: "Root title",
				preview: "Root preview expands to full width",
				snippet: "Matched snippet",
				summary_preview: null,
				preview_source: "fallback",
				last_activity_ts: "1700000004.000001",
				reply_count: 3,
				participant_count: 2,
				reaction_count: 1,
				file_count: 0,
				score: 2,
			},
			{},
		);

		expect(card.preview).toBe("Root preview expands to full width");
		expect(card.previewSource).toBe("fallback");
		expect(card.lastActivityTs).toBe("1700000004.000001");
	});

	it("prefers the ai summary preview for saved and starred cards", () => {
		const saved = threadCardDataFromSavedItem({
			id: "C123:1700000000.000001",
			thread_id: "C123:1700000000.000001",
			channel_id: "C123",
			channel_name: "general",
			author: null,
			root_ts: "1700000000.000001",
			title: "Root title",
			preview: "Root preview",
			summary_preview: "AI preview",
			preview_source: "ai",
			reply_count: 3,
			participant_count: 2,
			reaction_count: 1,
			file_count: 0,
			last_activity_ts: "1700000001.000001",
			saved_at: "1700000002",
		});
		const starred = threadCardDataFromStarredItem({
			id: "C123:1700000000.000001",
			thread_id: "C123:1700000000.000001",
			channel_id: "C123",
			channel_name: "general",
			author: null,
			root_ts: "1700000000.000001",
			title: "Root title",
			preview: "Root preview",
			summary_preview: "AI preview",
			preview_source: "ai",
			reply_count: 3,
			participant_count: 2,
			reaction_count: 1,
			file_count: 0,
			last_activity_ts: "1700000001.000001",
			pinned_at: "1700000002",
		});

		expect(saved.preview).toBe("AI preview");
		expect(starred.preview).toBe("AI preview");
		expect(saved.previewSource).toBe("ai");
		expect(starred.previewSource).toBe("ai");
	});
});

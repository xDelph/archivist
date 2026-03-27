import {
	normalizeSearchThreadSort,
	normalizeThreadListSort,
	sortThreadList,
	toSearchThreadSortSearch,
	toThreadListSortSearch,
} from "@/lib/thread-list-sort";

describe("thread list sort helpers", () => {
	const items = [
		{
			id: "alpha",
			last_activity_ts: "1700000100.000001",
			root_ts: "1700000000.000001",
			reply_count: 4,
			reaction_count: 1,
			participant_count: 2,
		},
		{
			id: "beta",
			last_activity_ts: "1700000200.000001",
			root_ts: "1700000001.000001",
			reply_count: 1,
			reaction_count: 5,
			participant_count: 3,
		},
		{
			id: "gamma",
			last_activity_ts: "1700000050.000001",
			root_ts: "1700000002.000001",
			reply_count: 2,
			reaction_count: 2,
			participant_count: 6,
		},
	];

	it("normalizes shared sort params and omits defaults from the url", () => {
		expect(normalizeThreadListSort(undefined)).toBe("date");
		expect(normalizeThreadListSort("invalid")).toBe("date");
		expect(normalizeSearchThreadSort(undefined)).toBe("relevance");
		expect(normalizeSearchThreadSort("invalid")).toBe("relevance");
		expect(toThreadListSortSearch("date")).toBeUndefined();
		expect(toThreadListSortSearch("replies")).toBe("replies");
		expect(toSearchThreadSortSearch("relevance")).toBeUndefined();
		expect(toSearchThreadSortSearch("people")).toBe("people");
	});

	it("sorts lists by the requested metric with stable date tie-breakers", () => {
		expect(
			sortThreadList(items, "date", (item) => ({
				id: item.id,
				lastActivityTs: item.last_activity_ts,
				rootTs: item.root_ts,
				replyCount: item.reply_count,
				reactionCount: item.reaction_count,
				participantCount: item.participant_count,
			})).map((item) => item.id),
		).toEqual(["beta", "alpha", "gamma"]);

		expect(
			sortThreadList(items, "replies", (item) => ({
				id: item.id,
				lastActivityTs: item.last_activity_ts,
				rootTs: item.root_ts,
				replyCount: item.reply_count,
				reactionCount: item.reaction_count,
				participantCount: item.participant_count,
			})).map((item) => item.id),
		).toEqual(["alpha", "gamma", "beta"]);

		expect(
			sortThreadList(items, "reactions", (item) => ({
				id: item.id,
				lastActivityTs: item.last_activity_ts,
				rootTs: item.root_ts,
				replyCount: item.reply_count,
				reactionCount: item.reaction_count,
				participantCount: item.participant_count,
			})).map((item) => item.id),
		).toEqual(["beta", "gamma", "alpha"]);

		expect(
			sortThreadList(items, "people", (item) => ({
				id: item.id,
				lastActivityTs: item.last_activity_ts,
				rootTs: item.root_ts,
				replyCount: item.reply_count,
				reactionCount: item.reaction_count,
				participantCount: item.participant_count,
			})).map((item) => item.id),
		).toEqual(["gamma", "beta", "alpha"]);
	});
});

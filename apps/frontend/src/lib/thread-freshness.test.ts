import {
	compareSlackTimestamps,
	getFreshestKnownThreadActivityTs,
	getThreadObservedActivityTs,
	shouldRefreshThreadDetail,
} from "@/lib/thread-freshness";

describe("thread freshness", () => {
	it("orders slack timestamps with and without fractions", () => {
		expect(compareSlackTimestamps("1773800000.000001", "1773800000")).toBe(1);
		expect(compareSlackTimestamps("1773800001", "1773800000.999999")).toBe(1);
	});

	it("picks the freshest known thread activity from cached lists", () => {
		expect(
			getFreshestKnownThreadActivityTs("C123:1773800000.000001", [
				{
					pages: [
						{
							items: [
								{
									id: "C123:1773800000.000001",
									last_activity_ts: "1773800100.000001",
								},
							],
						},
					],
				},
				{
					items: [
						{
							thread_id: "C123:1773800000.000001",
							last_activity_ts: "1773800200.000001",
						},
					],
				},
			]),
		).toBe("1773800200.000001");
	});

	it("uses the latest message timestamp when thread metadata is missing it", () => {
		expect(
			getThreadObservedActivityTs({
				root_ts: "1773800000.000001",
				last_activity_ts: null,
				messages: [{ ts: "1773800300.000001" }, { ts: "1773800100.000001" }],
			} as never),
		).toBe("1773800300.000001");
	});

	it("refreshes only when cached detail lags behind known activity", () => {
		expect(
			shouldRefreshThreadDetail(
				{
					root_ts: "1773800000.000001",
					last_activity_ts: "1773800100.000001",
					messages: [{ ts: "1773800100.000001" }],
				} as never,
				"1773800200.000001",
			),
		).toBe(true);

		expect(
			shouldRefreshThreadDetail(
				{
					root_ts: "1773800000.000001",
					last_activity_ts: "1773800200.000001",
					messages: [{ ts: "1773800200.000001" }],
				} as never,
				"1773800200.000001",
			),
		).toBe(false);
	});
});

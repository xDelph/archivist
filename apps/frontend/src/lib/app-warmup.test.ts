import { buildHomeWarmTargets, getHomeWarmTargetKey } from "@/lib/app-warmup";
import { describe, expect, it } from "vitest";

describe("app warmup helpers", () => {
	it("warms the inactive catch-up tabs for the current home filter", () => {
		expect(
			buildHomeWarmTargets("/", "?tab=steady&channel=C123&sort=reactions"),
		).toEqual([
			{ tab: "starred", channelId: "C123" },
			{
				tab: "fresh",
				window: "24h",
				sort: "reactions",
				channelId: "C123",
			},
		]);
	});

	it("warms all default catch-up tabs outside the home view", () => {
		expect(buildHomeWarmTargets("/saved", "")).toEqual([
			{ tab: "starred", channelId: undefined },
			{ tab: "fresh", window: "24h", sort: "date", channelId: undefined },
			{ tab: "steady", window: "7d", sort: "date", channelId: undefined },
		]);
	});

	it("builds stable warmup keys", () => {
		expect(
			getHomeWarmTargetKey({
				tab: "starred",
				channelId: "C123",
			}),
		).toBe("starred:C123");
		expect(
			getHomeWarmTargetKey({
				tab: "steady",
				window: "7d",
				sort: "reactions",
				channelId: "C123",
			}),
		).toBe("7d:reactions:C123");
	});
});

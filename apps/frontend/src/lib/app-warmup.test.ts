import { buildHomeWarmTargets, getHomeWarmTargetKey } from "@/lib/app-warmup";
import { describe, expect, it } from "vitest";

describe("app warmup helpers", () => {
	it("warms the inactive catch-up tabs for the current home filter", () => {
		expect(buildHomeWarmTargets("/", "?tab=steady&channel=C123")).toEqual([
			{ tab: "highlights", channelId: "C123" },
			{ tab: "fresh", window: "24h", channelId: "C123" },
			{ window: "7d", sort: "trending", channelId: "C123", tab: "trending" },
		]);
	});

	it("warms all default catch-up tabs outside the home view", () => {
		expect(buildHomeWarmTargets("/saved", "")).toEqual([
			{ tab: "highlights", channelId: undefined },
			{ tab: "fresh", window: "24h", channelId: undefined },
			{ tab: "steady", window: "7d", channelId: undefined },
			{ tab: "trending", window: "7d", sort: "trending", channelId: undefined },
		]);
	});

	it("builds stable warmup keys", () => {
		expect(
			getHomeWarmTargetKey({
				tab: "highlights",
				channelId: "C123",
			}),
		).toBe("highlights:C123");
		expect(
			getHomeWarmTargetKey({
				tab: "trending",
				window: "7d",
				sort: "trending",
				channelId: "C123",
			}),
		).toBe("7d:trending:C123");
	});
});

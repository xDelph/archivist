import {
	buildCatchUpWarmTargets,
	getCatchUpWarmTargetKey,
} from "@/lib/app-warmup";
import { describe, expect, it } from "vitest";

describe("app warmup helpers", () => {
	it("warms the inactive catch-up tabs for the current home filter", () => {
		expect(buildCatchUpWarmTargets("/", "?tab=steady&channel=C123")).toEqual([
			{ window: "24h", channelId: "C123" },
			{ window: "7d", sort: "trending", channelId: "C123" },
		]);
	});

	it("warms all default catch-up tabs outside the home view", () => {
		expect(buildCatchUpWarmTargets("/saved", "")).toEqual([
			{ window: "24h", channelId: undefined },
			{ window: "7d", channelId: undefined },
			{ window: "7d", sort: "trending", channelId: undefined },
		]);
	});

	it("builds stable warmup keys", () => {
		expect(
			getCatchUpWarmTargetKey({
				window: "7d",
				sort: "trending",
				channelId: "C123",
			}),
		).toBe("7d:trending:C123");
	});
});

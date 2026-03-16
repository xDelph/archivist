import {
	THREAD_SWIPE_ACTION_WIDTH,
	clampSwipeOffset,
	resolveSwipeOffset,
} from "@/lib/thread-swipe";

describe("thread swipe helpers", () => {
	it("clamps offsets to the configured action widths", () => {
		expect(clampSwipeOffset(160, THREAD_SWIPE_ACTION_WIDTH, 0)).toBe(
			THREAD_SWIPE_ACTION_WIDTH,
		);
		expect(clampSwipeOffset(-160, 0, THREAD_SWIPE_ACTION_WIDTH)).toBe(
			-THREAD_SWIPE_ACTION_WIDTH,
		);
		expect(clampSwipeOffset(42, THREAD_SWIPE_ACTION_WIDTH, 0)).toBe(42);
	});

	it("opens only when the swipe passes the snap threshold", () => {
		expect(
			resolveSwipeOffset(
				56,
				THREAD_SWIPE_ACTION_WIDTH,
				THREAD_SWIPE_ACTION_WIDTH,
			),
		).toBe(THREAD_SWIPE_ACTION_WIDTH);
		expect(
			resolveSwipeOffset(
				-56,
				THREAD_SWIPE_ACTION_WIDTH,
				THREAD_SWIPE_ACTION_WIDTH,
			),
		).toBe(-THREAD_SWIPE_ACTION_WIDTH);
		expect(
			resolveSwipeOffset(
				24,
				THREAD_SWIPE_ACTION_WIDTH,
				THREAD_SWIPE_ACTION_WIDTH,
			),
		).toBe(0);
		expect(
			resolveSwipeOffset(
				-24,
				THREAD_SWIPE_ACTION_WIDTH,
				THREAD_SWIPE_ACTION_WIDTH,
			),
		).toBe(0);
	});
});

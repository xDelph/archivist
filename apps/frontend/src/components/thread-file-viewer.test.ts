import { resolveFileViewerSwipeDirection } from "@/components/thread-file-viewer";

describe("thread file viewer mobile adaptation", () => {
	it("navigates backward on strong right swipes", () => {
		expect(
			resolveFileViewerSwipeDirection({
				deltaX: 72,
				deltaY: 10,
				currentIndex: 1,
				fileCount: 3,
			}),
		).toBe("prev");
	});

	it("navigates forward on strong left swipes", () => {
		expect(
			resolveFileViewerSwipeDirection({
				deltaX: -80,
				deltaY: 12,
				currentIndex: 0,
				fileCount: 3,
			}),
		).toBe("next");
	});

	it("ignores short or mostly vertical gestures", () => {
		expect(
			resolveFileViewerSwipeDirection({
				deltaX: 24,
				deltaY: 6,
				currentIndex: 1,
				fileCount: 3,
			}),
		).toBeNull();
		expect(
			resolveFileViewerSwipeDirection({
				deltaX: -70,
				deltaY: 90,
				currentIndex: 1,
				fileCount: 3,
			}),
		).toBeNull();
	});
});

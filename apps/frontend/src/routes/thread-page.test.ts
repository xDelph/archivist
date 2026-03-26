import { getNextVisibleTranscriptCount } from "@/routes/thread-page";

describe("thread transcript pagination", () => {
	it("grows the visible transcript in fixed lazy batches", () => {
		expect(getNextVisibleTranscriptCount(4, 30)).toBe(16);
		expect(getNextVisibleTranscriptCount(16, 30)).toBe(28);
	});

	it("never grows past the available message count", () => {
		expect(getNextVisibleTranscriptCount(28, 30)).toBe(30);
		expect(getNextVisibleTranscriptCount(4, 6)).toBe(6);
	});
});

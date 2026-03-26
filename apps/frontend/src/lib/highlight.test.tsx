import { highlightMatches } from "@/lib/highlight";

describe("highlightMatches", () => {
	it("handles missing text without throwing", () => {
		expect(() => highlightMatches(undefined, "rtk")).not.toThrow();
		expect(highlightMatches(undefined, "rtk")).toBe("");
	});
});

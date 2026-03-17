import { normalizeHomeTab, toHomeTabSearch } from "@/lib/home-tabs";

describe("home tab search params", () => {
	it("falls back to highlights when the search param is missing or invalid", () => {
		expect(normalizeHomeTab(undefined)).toBe("highlights");
		expect(normalizeHomeTab("invalid")).toBe("highlights");
	});

	it("omits the default highlights tab from the url", () => {
		expect(toHomeTabSearch("highlights")).toBeUndefined();
		expect(toHomeTabSearch("fresh")).toBe("fresh");
		expect(toHomeTabSearch("steady")).toBe("steady");
	});
});

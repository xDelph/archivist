import { normalizeHomeTab, toHomeTabSearch } from "@/lib/home-tabs";

describe("home tab search params", () => {
	it("falls back to starred when the search param is missing or invalid", () => {
		expect(normalizeHomeTab(undefined)).toBe("starred");
		expect(normalizeHomeTab("invalid")).toBe("starred");
	});

	it("omits the default starred tab from the url", () => {
		expect(toHomeTabSearch("starred")).toBeUndefined();
		expect(toHomeTabSearch("fresh")).toBe("fresh");
		expect(toHomeTabSearch("steady")).toBe("steady");
	});
});

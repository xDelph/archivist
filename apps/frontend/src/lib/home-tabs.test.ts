import { normalizeHomeTab, toHomeTabSearch } from "@/lib/home-tabs";

describe("home tab search params", () => {
	it("falls back to fresh when the search param is missing or invalid", () => {
		expect(normalizeHomeTab(undefined)).toBe("fresh");
		expect(normalizeHomeTab("invalid")).toBe("fresh");
	});

	it("omits the default fresh tab from the url", () => {
		expect(toHomeTabSearch("fresh")).toBeUndefined();
		expect(toHomeTabSearch("steady")).toBe("steady");
	});
});

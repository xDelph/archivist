import { resolveHomeEntryTarget } from "@/lib/app-entry-loading";

describe("app entry loading", () => {
	it("uses the default starred landing state on the home route", () => {
		expect(resolveHomeEntryTarget("/", "")).toEqual({
			tab: "starred",
			channelId: undefined,
			sort: "date",
		});
	});

	it("resolves fresh tab filters for the initial home query", () => {
		expect(
			resolveHomeEntryTarget("/", "?tab=fresh&channel=C123&sort=reactions"),
		).toEqual({
			tab: "fresh",
			channelId: "C123",
			sort: "reactions",
			window: "24h",
		});
	});

	it("ignores non-home routes", () => {
		expect(resolveHomeEntryTarget("/saved", "")).toBeNull();
	});
});

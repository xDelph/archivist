import { getSearchFilterSummary } from "@/routes/search-page";

describe("search filter disclosure helpers", () => {
	const defaultDateRange = {
		from: "2026-03-23",
		to: "2026-03-24",
	};

	it("keeps filters collapsed by default when no filter deviates", () => {
		expect(
			getSearchFilterSummary({
				channelId: "",
				dateFrom: defaultDateRange.from,
				dateTo: defaultDateRange.to,
				defaultDateRange,
			}),
		).toEqual({
			activeCount: 0,
			hasActiveFilters: false,
		});
	});

	it("opens filters when any non-default filter is active", () => {
		expect(
			getSearchFilterSummary({
				channelId: "C123",
				dateFrom: defaultDateRange.from,
				dateTo: "2026-03-20",
				defaultDateRange,
			}),
		).toEqual({
			activeCount: 2,
			hasActiveFilters: true,
		});
	});
});

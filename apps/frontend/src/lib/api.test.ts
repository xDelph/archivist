import { buildApiUrl, resolveApiBaseUrl } from "@/lib/api";

describe("api helpers", () => {
	it("normalizes base urls", () => {
		expect(resolveApiBaseUrl("http://localhost:4000/")).toBe(
			"http://localhost:4000",
		);
	});

	it("keeps relative api urls by default", () => {
		expect(resolveApiBaseUrl("")).toBe("");
		expect(buildApiUrl("/health", "")).toBe("/health");
	});

	it("builds absolute paths", () => {
		expect(buildApiUrl("health", "http://localhost:4000")).toBe(
			"http://localhost:4000/health",
		);
	});
});

import { describe, expect, it } from "vitest";

import { isImageFile } from "./thread-files";

describe("isImageFile", () => {
	it("detects image mime types", () => {
		expect(isImageFile("image/png")).toBe(true);
		expect(isImageFile("application/pdf")).toBe(false);
		expect(isImageFile(null)).toBe(false);
	});
});

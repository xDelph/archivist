import { describe, expect, it } from "vitest";

import { App } from "./app";

describe("App", () => {
	it("exports the landing application component", () => {
		expect(App).toBeTypeOf("function");
	});
});

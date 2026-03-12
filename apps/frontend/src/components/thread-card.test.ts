import { shouldRenderThreadPreview } from "@/components/thread-card";
import { buildThreadMetrics } from "@/components/thread-metrics";
import { renderSlackTextWithHighlights } from "@/lib/thread-display";

describe("thread card preview visibility", () => {
	it("hides previews that duplicate the title text", () => {
		expect(
			shouldRenderThreadPreview(
				"gerber je sais pas , mais c'est relou",
				"gerber je sais pas , mais c'est relou",
			),
		).toBe(false);
	});

	it("hides previews that only differ by rich rendering", () => {
		expect(
			shouldRenderThreadPreview(
				renderSlackTextWithHighlights(
					"Read <https://example.com/docs|the docs> :fire:",
				),
				renderSlackTextWithHighlights(
					"Read <https://example.com/docs|the docs> :fire:",
				),
			),
		).toBe(false);
	});

	it("keeps previews when they add more context than the title", () => {
		expect(
			shouldRenderThreadPreview(
				"Claude Code Ultimate Guide",
				"Claude Code Ultimate Guide recap fev-mars 2026",
			),
		).toBe(true);
	});

	it("always shows the reply count metric", () => {
		expect(
			buildThreadMetrics(0, 4, 2, 0).map((metric) => metric.value),
		).toEqual([0, 4, 2]);
	});
});

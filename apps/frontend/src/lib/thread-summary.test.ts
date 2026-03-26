import { resolveThreadSummaryPresentation } from "@/lib/thread-summary";

describe("thread summary presentation", () => {
	it("prefers ai summary text and takeaway", () => {
		expect(
			resolveThreadSummaryPresentation({
				title: "Fallback title",
				preview: "Fallback preview",
				summary: {
					text: "AI summary",
					full_summary: "## AI summary",
					why_it_mattered: "Useful takeaway",
					status: "answered",
					topic_tags: ["launch"],
					model: "test-model",
					generated_at: 42,
					is_stale: false,
					source: "ai",
				},
				messages: [{ text: "Root message" }],
			}),
		).toEqual({
			body: "## AI summary",
			bodyFormat: "markdown",
			secondary: "Useful takeaway",
			source: "ai",
		});
	});

	it("falls back to preview when no full summary is available", () => {
		expect(
			resolveThreadSummaryPresentation({
				title: "Launch checklist",
				preview: " launch   checklist ",
				summary: {
					text: "Launch checklist",
					full_summary: null,
					why_it_mattered: null,
					status: null,
					topic_tags: [],
					model: null,
					generated_at: null,
					is_stale: false,
					source: "fallback",
				},
				messages: [{ text: "Root message" }],
			}),
		).toEqual({
			body: "Launch checklist",
			bodyFormat: "plain",
			secondary: null,
			source: "fallback",
		});
	});

	it("falls back to a generic heading and root message when summary metadata is missing", () => {
		expect(
			resolveThreadSummaryPresentation({
				title: null,
				preview: null,
				summary: {
					text: null,
					full_summary: null,
					why_it_mattered: null,
					status: null,
					topic_tags: [],
					model: null,
					generated_at: null,
					is_stale: false,
					source: "none",
				},
				messages: [{ text: "Root message" }],
			}),
		).toEqual({
			body: "Root message",
			bodyFormat: "plain",
			secondary: null,
			source: "none",
		});
	});
});

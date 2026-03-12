import {
	extractLinks,
	groupReactions,
	renderSlackText,
	slackEmojiToUnicode,
} from "@/lib/thread-display";
import { renderToStaticMarkup } from "react-dom/server";

describe("thread display emoji helpers", () => {
	it("renders slack emoji shortcodes inside message text", () => {
		const markup = renderToStaticMarkup(
			renderSlackText("Nice one :+1::skin-tone-2: and :fire:"),
		);

		expect(markup).toContain("👍🏻");
		expect(markup).toContain("🔥");
	});

	it("normalizes grouped reactions to unicode", () => {
		expect(
			groupReactions([
				{ name: "+1", user_id: "U1" },
				{ name: "+1", user_id: "U2" },
				{ name: "+1::skin-tone-2", user_id: "U3" },
			]),
		).toEqual([
			{ name: "+1", count: 2, emoji: "👍" },
			{ name: "+1::skin-tone-2", count: 1, emoji: "👍🏻" },
		]);
	});

	it("maps explicit slack reaction names", () => {
		expect(slackEmojiToUnicode(":+1::skin-tone-2:")).toBe("👍🏻");
		expect(slackEmojiToUnicode(":thumbsup:")).toBe("👍");
	});

	it("renders slack links with their label and extracts them once", () => {
		const markup = renderToStaticMarkup(
			renderSlackText("Read <https://example.com/docs|the docs> today"),
		);

		expect(markup).toContain(">the docs<");
		expect(markup).not.toContain(">https://example.com/docs<");
		expect(
			extractLinks(
				"Read <https://example.com/docs|the docs> and https://example.com/docs",
			),
		).toEqual([{ href: "https://example.com/docs", label: "the docs" }]);
	});

	it("renders truncated slack links without leaking the raw angle-bracket syntax", () => {
		const markup = renderToStaticMarkup(
			renderSlackText(
				"Share <https://www.linkedin.com/feed/update/urn:li:activity:1234567890|LinkedIn post",
			),
		);

		expect(markup).toContain(">LinkedIn post<");
		expect(markup).not.toContain("&lt;https://www.linkedin.com/feed/update");
		expect(
			extractLinks(
				"Share <https://www.linkedin.com/feed/update/urn:li:activity:1234567890|LinkedIn post",
			),
		).toEqual([
			{
				href: "https://www.linkedin.com/feed/update/urn:li:activity:1234567890",
				label: "LinkedIn post",
			},
		]);
	});

	it("renders truncated slack links without labels as links", () => {
		const markup = renderToStaticMarkup(
			renderSlackText(
				"Reference <https://www.linkedin.com/feed/update/urn:li:activity:1234567890",
			),
		);

		expect(markup).toContain(
			">https://www.linkedin.com/feed/update/urn:li:activity:1234567890<",
		);
		expect(markup).not.toContain(
			"&lt;https://www.linkedin.com/feed/update/urn:li:activity:1234567890",
		);
	});
});

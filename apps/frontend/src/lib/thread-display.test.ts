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

	it("maps slack thread permalinks to Archivist thread routes", () => {
		const permalink =
			"https://devwithai.slack.com/archives/C08KYNHH3D0/p1769500777510479?thread_ts=1769429252.836289&cid=C08KYNHH3D0";
		const markup = renderToStaticMarkup(renderSlackText(`See ${permalink}`));

		expect(markup).toContain('href="/threads/C08KYNHH3D0%3A1769429252.836289"');
		expect(markup).not.toContain('target="_blank"');
		expect(extractLinks(`See ${permalink}`)).toEqual([
			{
				href: permalink,
				label: null,
				appHref: "/threads/C08KYNHH3D0%3A1769429252.836289",
			},
		]);
	});

	it("falls back to the message permalink timestamp when no thread_ts exists", () => {
		const permalink =
			"https://devwithai.slack.com/archives/C08KYNHH3D0/p1769500777510479";

		expect(extractLinks(`See ${permalink}`)).toEqual([
			{
				href: permalink,
				label: null,
				appHref: "/threads/C08KYNHH3D0%3A1769500777.510479",
			},
		]);
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

	it("decodes html entities before rendering links and text", () => {
		const markup = renderToStaticMarkup(
			renderSlackText("Go --&gt; https://example.com?a=1&amp;b=2"),
		);

		expect(markup).toContain("--&gt;");
		expect(markup).toContain("https://example.com?a=1&amp;b=2");
		expect(markup).not.toContain("--&amp;gt;");
	});

	it("renders resolved mentions with the shared green accent treatment", () => {
		const markup = renderToStaticMarkup(
			renderSlackText("Ask @Thomas in #general today"),
		);

		expect(markup).toContain('class="font-medium text-[#20cb74]"');
		expect(markup).toContain("@Thomas");
		expect(markup).toContain("#general");
	});
});

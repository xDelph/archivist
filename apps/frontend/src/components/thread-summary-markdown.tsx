import { renderSlackText } from "@/lib/thread-display";
import type { ReactNode } from "react";

interface ThreadSummaryMarkdownProps {
	content: string;
}

export function ThreadSummaryMarkdown({ content }: ThreadSummaryMarkdownProps) {
	return <div className="space-y-3">{renderBlocks(content)}</div>;
}

function renderBlocks(content: string) {
	const lines = content.replace(/\r\n/g, "\n").split("\n");
	const blocks: ReactNode[] = [];
	let index = 0;

	while (index < lines.length) {
		const line = lines[index]?.trim();
		if (!line) {
			index += 1;
			continue;
		}

		const headingMatch = line.match(/^(#{1,3})\s+(.*)$/);
		if (headingMatch) {
			const level = headingMatch[1].length;
			const text = headingMatch[2];
			const className =
				level === 1
					? "text-base font-semibold text-(--color-text-primary)"
					: level === 2
						? "text-[0.95rem] font-semibold text-(--color-text-primary)"
						: "text-[0.88rem] font-semibold text-(--color-text-primary)";
			blocks.push(
				<div key={`heading-${index}`} className={className}>
					{renderInlineMarkdown(text)}
				</div>,
			);
			index += 1;
			continue;
		}

		const unorderedMatch = line.match(/^[-*]\s+(.*)$/);
		const orderedMatch = line.match(/^\d+\.\s+(.*)$/);
		if (unorderedMatch || orderedMatch) {
			const items: string[] = [];
			const ordered = Boolean(orderedMatch);
			while (index < lines.length) {
				const candidate = lines[index]?.trim();
				const match = ordered
					? candidate?.match(/^\d+\.\s+(.*)$/)
					: candidate?.match(/^[-*]\s+(.*)$/);
				if (!match) {
					break;
				}
				items.push(match[1]);
				index += 1;
			}

			const ListTag = ordered ? "ol" : "ul";
			const itemCounts = new Map<string, number>();
			blocks.push(
				<ListTag
					key={`list-${index}`}
					className={
						ordered
							? "ml-5 list-decimal space-y-1.5 text-[0.88rem] leading-6 text-(--color-text-secondary)"
							: "ml-5 list-disc space-y-1.5 text-[0.88rem] leading-6 text-(--color-text-secondary)"
					}
				>
					{items.map((item) => {
						const occurrence = itemCounts.get(item) ?? 0;
						itemCounts.set(item, occurrence + 1);

						return (
							<li
								key={`${ordered ? "ordered" : "unordered"}-${item}-${occurrence}`}
							>
								{renderInlineMarkdown(item)}
							</li>
						);
					})}
				</ListTag>,
			);
			continue;
		}

		const paragraphLines: string[] = [];
		while (index < lines.length) {
			const candidate = lines[index]?.trim();
			if (!candidate) {
				break;
			}
			if (
				/^(#{1,3})\s+/.test(candidate) ||
				/^[-*]\s+/.test(candidate) ||
				/^\d+\.\s+/.test(candidate)
			) {
				break;
			}
			paragraphLines.push(candidate);
			index += 1;
		}
		blocks.push(
			<p
				key={`paragraph-${index}`}
				className="text-[0.88rem] leading-6 text-(--color-text-secondary)"
			>
				{renderInlineMarkdown(paragraphLines.join(" "))}
			</p>,
		);
	}

	return blocks;
}

function renderInlineMarkdown(content: string): ReactNode[] {
	const tokens: ReactNode[] = [];
	const pattern = /(\[[^\]]+\]\([^)]+\)|\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g;
	let lastIndex = 0;

	for (const match of content.matchAll(pattern)) {
		if (match.index === undefined) {
			continue;
		}

		if (match.index > lastIndex) {
			tokens.push(renderSlackText(content.slice(lastIndex, match.index)));
		}

		const token = match[0];
		if (token.startsWith("[") && token.includes("](") && token.endsWith(")")) {
			const linkMatch = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
			if (linkMatch) {
				tokens.push(
					<a
						key={`${match.index}-link`}
						href={linkMatch[2]}
						target="_blank"
						rel="noreferrer"
						className="text-(--color-accent-soft) underline underline-offset-4"
					>
						{renderInlineMarkdown(linkMatch[1])}
					</a>,
				);
			}
		} else if (token.startsWith("**") && token.endsWith("**")) {
			tokens.push(
				<strong
					key={`${match.index}-strong`}
					className="font-semibold text-(--color-text-primary)"
				>
					{renderInlineMarkdown(token.slice(2, -2))}
				</strong>,
			);
		} else if (token.startsWith("*") && token.endsWith("*")) {
			tokens.push(
				<em key={`${match.index}-em`} className="italic">
					{renderInlineMarkdown(token.slice(1, -1))}
				</em>,
			);
		} else {
			tokens.push(
				<code
					key={`${match.index}-code`}
					className="rounded bg-(--color-bg-surface) px-1.5 py-0.5 font-mono text-[0.82em] text-(--color-text-primary)"
				>
					{token.slice(1, -1)}
				</code>,
			);
		}

		lastIndex = match.index + token.length;
	}

	if (lastIndex < content.length) {
		tokens.push(renderSlackText(content.slice(lastIndex)));
	}

	return tokens;
}

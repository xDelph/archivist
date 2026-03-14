import { ExternalLink, Link2 } from "lucide-react";

import { EmptyState } from "@/components/empty-state";
import { type LinkMetadata, fetchLinkMetadata } from "@/lib/api";
import type { SlackLink } from "@/lib/thread-display";
import { useQueries } from "@tanstack/react-query";

interface ThreadLinksPanelProps {
	links: SlackLink[];
}

export function ThreadLinksPanel({ links }: ThreadLinksPanelProps) {
	const previewQueries = useQueries({
		queries: links.map((link) => ({
			queryKey: ["link-metadata", link.href],
			queryFn: () => fetchLinkMetadata(link.href),
			enabled: /^https?:/i.test(link.href),
			staleTime: 5 * 60_000,
			retry: false,
		})),
	});

	if (!links.length) {
		return (
			<EmptyState
				title="No links found"
				description="This thread does not contain extractable URLs."
				icon={<ExternalLink className="size-5" />}
			/>
		);
	}

	return (
		<ul className="space-y-2 text-[0.82rem]">
			{links.map((link, index) => (
				<li key={link.href}>
					<LinkPreviewCard
						link={link}
						metadata={previewQueries[index]?.data}
						isPending={previewQueries[index]?.isPending ?? false}
					/>
				</li>
			))}
		</ul>
	);
}

function LinkPreviewCard({
	link,
	metadata,
	isPending,
}: {
	link: SlackLink;
	metadata: LinkMetadata | undefined;
	isPending: boolean;
}) {
	const host = displayHost(metadata?.url ?? link.href);
	const title = metadata?.title || link.label || host;
	const description = metadata?.description;
	const siteName = metadata?.site_name || host;
	const image = metadata?.image;
	const showPreview = Boolean(
		image || metadata?.title || description || metadata?.site_name,
	);

	return (
		<a
			href={link.appHref ?? link.href}
			target={link.appHref ? undefined : "_blank"}
			rel={link.appHref ? undefined : "noreferrer"}
			className="group flex items-stretch gap-3 overflow-hidden rounded-[0.9rem] border border-white/8 bg-[#0a0d0f] p-3 transition-colors hover:border-[#1fc86f]/20 hover:bg-[#0d1114]"
		>
			{showPreview ? (
				<div className="min-w-0 flex-1">
					<p className="text-[0.62rem] font-medium uppercase tracking-[0.18em] text-[#6f747c]">
						{siteName}
					</p>
					<p className="mt-1 text-[0.92rem] font-medium text-white group-hover:text-[#dff7e9]">
						{title}
					</p>
					{description ? (
						<p className="mt-1 line-clamp-2 text-[0.78rem] leading-5 text-[#9aa0a8]">
							{description}
						</p>
					) : isPending ? (
						<p className="mt-1 text-[0.78rem] text-[#6f747c]">
							Loading preview…
						</p>
					) : null}
					<div className="mt-2 inline-flex items-center gap-1.5 text-[0.74rem] text-[#5ea7ff]">
						<ExternalLink className="size-3.5" />
						<span className="truncate">{host}</span>
					</div>
				</div>
			) : (
				<div className="min-w-0 flex-1">
					<div className="inline-flex items-center gap-2 break-all text-[#5ea7ff] underline decoration-[#2d5cc2] underline-offset-3 group-hover:text-[#89bbff]">
						<Link2 className="size-4 shrink-0" />
						{link.label || link.href}
					</div>
				</div>
			)}
			{image ? (
				<div className="relative hidden h-24 w-28 shrink-0 overflow-hidden rounded-[0.85rem] border border-white/8 bg-[#050607] sm:block">
					<img src={image} alt={title} className="h-full w-full object-cover" />
				</div>
			) : null}
		</a>
	);
}

function displayHost(value: string) {
	try {
		return new URL(value).host.replace(/^www\./, "");
	} catch {
		return value;
	}
}

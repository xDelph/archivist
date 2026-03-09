import { ArrowRight } from "lucide-react";

interface RoadmapCardProps {
	eyebrow: string;
	title: string;
	description: string;
	items: string[];
}

export function RoadmapCard({
	eyebrow,
	title,
	description,
	items,
}: RoadmapCardProps) {
	return (
		<article className="rounded-(--radius-section) border border-(--color-border-subtle) bg-(--color-bg-surface)/60 p-6 shadow-[0_16px_48px_oklch(0.05_0.02_220/0.4)] backdrop-blur-sm">
			<p className="text-xs font-medium uppercase tracking-[0.28em] text-(--color-accent-soft)">
				{eyebrow}
			</p>
			<h3 className="mt-4 text-2xl font-semibold text-(--color-text-primary)">
				{title}
			</h3>
			<p className="mt-3 text-sm leading-relaxed text-(--color-text-secondary)">
				{description}
			</p>
			<ul className="mt-6 space-y-3">
				{items.map((item) => (
					<li
						key={item}
						className="flex items-start gap-3 text-sm text-(--color-text-primary)"
					>
						<ArrowRight className="mt-0.5 size-4 shrink-0 text-(--color-accent-soft)" />
						<span>{item}</span>
					</li>
				))}
			</ul>
		</article>
	);
}

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
		<article className="rounded-[2rem] border border-white/10 bg-white/6 p-6 shadow-[0_20px_50px_rgba(3,7,18,0.35)] backdrop-blur">
			<p className="text-xs uppercase tracking-[0.28em] text-[var(--accent-soft)]">
				{eyebrow}
			</p>
			<h3 className="mt-4 text-2xl font-semibold text-white">{title}</h3>
			<p className="mt-3 text-sm leading-6 text-slate-300">{description}</p>
			<ul className="mt-6 space-y-3">
				{items.map((item) => (
					<li
						key={item}
						className="flex items-start gap-3 text-sm text-slate-200"
					>
						<ArrowRight className="mt-0.5 size-4 text-[var(--accent-soft)]" />
						<span>{item}</span>
					</li>
				))}
			</ul>
		</article>
	);
}

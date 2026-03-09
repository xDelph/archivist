import { cn } from "@/lib/utils";
import type { PropsWithChildren, ReactNode } from "react";

interface SectionCardProps extends PropsWithChildren {
	title: string;
	description?: string;
	eyebrow?: string;
	actions?: ReactNode;
	className?: string;
}

export function SectionCard({
	title,
	description,
	eyebrow,
	actions,
	className,
	children,
}: SectionCardProps) {
	return (
		<section
			className={cn(
				"rounded-[2rem] border border-white/10 bg-white/[0.045] p-5 shadow-[0_20px_60px_rgba(5,9,20,0.26)] backdrop-blur",
				className,
			)}
		>
			<div className="flex items-start justify-between gap-4">
				<div>
					{eyebrow ? (
						<p className="text-[0.65rem] uppercase tracking-[0.28em] text-[var(--accent-soft)]">
							{eyebrow}
						</p>
					) : null}
					<h2 className="mt-2 text-xl font-semibold text-white">{title}</h2>
					{description ? (
						<p className="mt-2 text-sm leading-6 text-slate-300">
							{description}
						</p>
					) : null}
				</div>
				{actions}
			</div>
			<div className="mt-5">{children}</div>
		</section>
	);
}

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
				"rounded-(--radius-section) border border-(--color-border-subtle) bg-(--color-bg-surface)/60 p-5 shadow-[0_16px_48px_oklch(0.05_0.02_220/0.4)] backdrop-blur-sm sm:p-6",
				className,
			)}
		>
			<div className="flex items-start justify-between gap-4">
				<div className="min-w-0">
					{eyebrow ? (
						<p className="text-[0.62rem] font-medium uppercase tracking-[0.3em] text-(--color-accent-soft)">
							{eyebrow}
						</p>
					) : null}
					<h2 className="mt-2 text-lg font-semibold text-(--color-text-primary) sm:text-xl">
						{title}
					</h2>
					{description ? (
						<p className="mt-2 text-sm leading-relaxed text-(--color-text-secondary)">
							{description}
						</p>
					) : null}
				</div>
				{actions ? <div className="shrink-0">{actions}</div> : null}
			</div>
			<div className="mt-5">{children}</div>
		</section>
	);
}

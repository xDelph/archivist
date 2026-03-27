import { cn } from "@/lib/utils";
import type { PropsWithChildren, ReactNode } from "react";

interface SectionCardProps extends PropsWithChildren {
	title: ReactNode;
	titleClassName?: string;
	description?: string;
	eyebrow?: string;
	actions?: ReactNode;
	toolbar?: ReactNode;
	overlayActions?: boolean;
	className?: string;
}

export function SectionCard({
	title,
	titleClassName,
	description,
	eyebrow,
	actions,
	toolbar,
	overlayActions = false,
	className,
	children,
}: SectionCardProps) {
	return (
		<section className={cn("surface-panel p-4 sm:p-4.5", className)}>
			<div
				className={cn(
					"flex items-start justify-between gap-4",
					overlayActions && "relative block",
				)}
			>
				<div className="min-w-0">
					{eyebrow ? (
						<p className="text-eyebrow text-[0.72rem] font-medium uppercase tracking-[0.18em]">
							{eyebrow}
						</p>
					) : null}
					<h2
						className={cn(
							"mt-1.5 max-w-[26ch] text-[clamp(1.1rem,2vw,1.35rem)] font-semibold leading-tight tracking-tight text-(--color-text-primary)",
							titleClassName,
						)}
					>
						{title}
					</h2>
					{description ? (
						<p className="text-copy-muted mt-1.5 max-w-[70ch] text-[0.92rem] leading-relaxed">
							{description}
						</p>
					) : null}
				</div>
				{actions ? (
					<div
						className={cn(
							"shrink-0",
							overlayActions && "absolute right-0 top-0 z-10",
						)}
					>
						{actions}
					</div>
				) : null}
			</div>
			{toolbar ? <div className="mt-3">{toolbar}</div> : null}
			<div className={toolbar ? "mt-3" : "mt-3.5"}>{children}</div>
		</section>
	);
}

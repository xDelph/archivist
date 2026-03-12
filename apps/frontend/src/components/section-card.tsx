import { cn } from "@/lib/utils";
import type { PropsWithChildren, ReactNode } from "react";

interface SectionCardProps extends PropsWithChildren {
	title: ReactNode;
	titleClassName?: string;
	description?: string;
	eyebrow?: string;
	actions?: ReactNode;
	overlayActions?: boolean;
	className?: string;
}

export function SectionCard({
	title,
	titleClassName,
	description,
	eyebrow,
	actions,
	overlayActions = false,
	className,
	children,
}: SectionCardProps) {
	return (
		<section
			className={cn(
				"rounded-[0.82rem] border border-white/8 bg-[#07090b] p-3 shadow-[0_8px_24px_rgba(0,0,0,0.18)] sm:p-3.5",
				className,
			)}
		>
			<div
				className={cn(
					"flex items-start justify-between gap-3",
					overlayActions && "relative block",
				)}
			>
				<div className="min-w-0">
					{eyebrow ? (
						<p className="text-[0.58rem] font-medium uppercase tracking-[0.25em] text-[#20cb74]">
							{eyebrow}
						</p>
					) : null}
					<h2
						className={cn(
							"mt-1 text-[1.02rem] font-semibold tracking-tight text-white sm:text-[1.14rem]",
							titleClassName,
						)}
					>
						{title}
					</h2>
					{description ? (
						<p className="mt-1 text-[0.76rem] leading-relaxed text-[#92959c]">
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
			<div className="mt-2.5">{children}</div>
		</section>
	);
}

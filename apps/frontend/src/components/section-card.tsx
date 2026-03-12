import { cn } from "@/lib/utils";
import type { PropsWithChildren, ReactNode } from "react";

interface SectionCardProps extends PropsWithChildren {
	title: ReactNode;
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
				"rounded-[1.75rem] border border-white/8 bg-[#090b0d] p-5 shadow-[0_18px_56px_rgba(0,0,0,0.36)] backdrop-blur-sm sm:p-6",
				className,
			)}
		>
			<div className="flex items-start justify-between gap-4">
				<div className="min-w-0">
					{eyebrow ? (
						<p className="text-[0.68rem] font-medium uppercase tracking-[0.3em] text-[#20cb74]">
							{eyebrow}
						</p>
					) : null}
					<h2 className="mt-2 text-xl font-semibold tracking-tight text-white">
						{title}
					</h2>
					{description ? (
						<p className="mt-2 text-sm leading-relaxed text-[#92959c]">
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

import type { ReactNode } from "react";

interface EmptyStateProps {
	title: string;
	description: string;
	icon?: ReactNode;
}

export function EmptyState({ title, description, icon }: EmptyStateProps) {
	return (
		<div className="rounded-[1.3rem] border border-dashed border-white/12 bg-[#07090b] px-5 py-10 text-center">
			{icon ? (
				<div className="mx-auto mb-4 flex size-10 items-center justify-center rounded-full bg-[#18cc77]/12 text-[#20cb74]">
					{icon}
				</div>
			) : null}
			<h3 className="text-base font-semibold text-white">{title}</h3>
			<p className="mx-auto mt-2 max-w-sm text-sm leading-relaxed text-[#8e929a]">
				{description}
			</p>
		</div>
	);
}

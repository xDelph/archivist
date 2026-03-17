import type { ReactNode } from "react";

interface EmptyStateProps {
	title: string;
	description: string;
	icon?: ReactNode;
}

export function EmptyState({ title, description, icon }: EmptyStateProps) {
	return (
		<div className="surface-empty px-5 py-10 text-center">
			{icon ? (
				<div className="accent-icon-shell mx-auto mb-4 flex size-10 items-center justify-center rounded-full">
					{icon}
				</div>
			) : null}
			<h3 className="text-base font-semibold text-(--color-text-primary)">
				{title}
			</h3>
			<p className="text-copy-soft mx-auto mt-2 max-w-sm text-sm leading-relaxed">
				{description}
			</p>
		</div>
	);
}

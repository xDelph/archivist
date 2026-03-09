import type { ReactNode } from "react";

interface EmptyStateProps {
	title: string;
	description: string;
	icon?: ReactNode;
}

export function EmptyState({ title, description, icon }: EmptyStateProps) {
	return (
		<div className="rounded-(--radius-card) border border-dashed border-(--color-border-default) bg-(--color-bg-base)/40 px-5 py-10 text-center">
			{icon ? (
				<div className="mx-auto mb-4 flex size-10 items-center justify-center rounded-full bg-(--color-accent-soft)/10 text-(--color-accent-soft)">
					{icon}
				</div>
			) : null}
			<h3 className="text-base font-semibold text-(--color-text-primary)">
				{title}
			</h3>
			<p className="mx-auto mt-2 max-w-sm text-sm leading-relaxed text-(--color-text-secondary)">
				{description}
			</p>
		</div>
	);
}

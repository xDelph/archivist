interface EmptyStateProps {
	title: string;
	description: string;
}

export function EmptyState({ title, description }: EmptyStateProps) {
	return (
		<div className="rounded-[1.5rem] border border-dashed border-white/12 bg-slate-950/30 px-4 py-10 text-center">
			<h3 className="text-lg font-semibold text-white">{title}</h3>
			<p className="mx-auto mt-3 max-w-md text-sm leading-6 text-slate-300">
				{description}
			</p>
		</div>
	);
}

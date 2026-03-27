import { cn } from "@/lib/utils";

interface ThreadListSortBarProps<T extends string> {
	className?: string;
	label?: string;
	onChange: (value: T) => void;
	options: readonly {
		label: string;
		value: T;
	}[];
	value: T;
}

export function ThreadListSortBar<T extends string>({
	className,
	label = "Sort",
	onChange,
	options,
	value,
}: ThreadListSortBarProps<T>) {
	return (
		<div
			className={cn(
				"flex min-w-0 items-center gap-3 overflow-hidden",
				className,
			)}
		>
			<p className="text-copy-quiet shrink-0 text-[0.68rem] font-medium uppercase tracking-[0.18em] sm:text-[0.72rem]">
				{label}
			</p>
			<div className="min-w-0 flex-1 overflow-x-auto px-0.5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
				<div className="flex min-w-max items-center gap-1.5">
					{options.map((option) => {
						const isActive = option.value === value;

						return (
							<button
								key={option.value}
								type="button"
								aria-pressed={isActive}
								onClick={() => onChange(option.value)}
								className={cn(
									"shrink-0 rounded-[0.8rem] border px-3 py-1.5 text-[0.8rem] font-medium transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40 active:scale-[0.99]",
									isActive
										? "border-(--color-border-accent) bg-(--color-accent)/12 text-(--color-accent-soft)"
										: "surface-ghost surface-ghost-hover text-(--color-text-secondary)",
								)}
							>
								{option.label}
							</button>
						);
					})}
				</div>
			</div>
		</div>
	);
}

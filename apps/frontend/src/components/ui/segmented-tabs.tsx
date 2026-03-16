import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export interface SegmentedTabItem<T extends string> {
	key: T;
	label: string;
	shortLabel?: string;
	icon: ReactNode;
	disabled?: boolean;
}

interface SegmentedTabsProps<T extends string> {
	items: readonly SegmentedTabItem<T>[];
	value: T;
	onChange: (value: T) => void;
	className?: string;
}

export function SegmentedTabs<T extends string>({
	items,
	value,
	onChange,
	className,
}: SegmentedTabsProps<T>) {
	return (
		<div
			className={cn(
				"surface-panel surface-panel-soft flex w-full items-center gap-1 p-1.5 sm:gap-1.5 sm:p-1.5",
				className,
			)}
		>
			{items.map((item) => (
				<button
					key={item.key}
					type="button"
					onClick={() => onChange(item.key)}
					disabled={item.disabled}
					className={cn(
						"flex min-h-11 min-w-0 flex-1 items-center justify-center gap-1 px-1.5 py-2 text-[0.74rem] font-medium transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out sm:min-h-10 sm:gap-2 sm:rounded-[0.75rem] sm:px-3 sm:text-[0.84rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40 active:translate-y-px",
						value === item.key
							? "rounded-[0.7rem] border-(--color-border-accent) bg-(--color-accent-soft)/14 text-(--color-accent-soft) shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
							: "hover-surface-soft text-(--color-text-secondary)",
						item.disabled &&
							"cursor-not-allowed opacity-40 active:translate-y-0 hover:bg-transparent hover:text-(--color-text-secondary)",
					)}
				>
					{item.icon}
					<span className="truncate sm:hidden">
						{item.shortLabel ?? item.label}
					</span>
					<span className="hidden truncate sm:inline">{item.label}</span>
				</button>
			))}
		</div>
	);
}

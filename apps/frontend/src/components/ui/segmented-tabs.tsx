import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export interface SegmentedTabItem<T extends string> {
	key: T;
	label: string;
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
				"surface-panel surface-panel-soft flex w-full items-center gap-1 p-1.5 sm:gap-2 sm:p-2",
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
						"flex min-w-0 flex-1 items-center justify-center gap-1 rounded-[0.6rem] px-1 py-1.5 text-[0.62rem] font-medium transition-colors sm:gap-2 sm:px-3 sm:py-2 sm:text-[0.8rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40",
						value === item.key
							? "bg-(--color-accent-soft)/15 text-(--color-accent-soft)"
							: "hover-surface-soft text-(--color-text-secondary)",
						item.disabled &&
							"cursor-not-allowed opacity-40 hover:bg-transparent hover:text-(--color-text-secondary)",
					)}
				>
					{item.icon}
					<span className="truncate">{item.label}</span>
				</button>
			))}
		</div>
	);
}

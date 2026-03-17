import { cn } from "@/lib/utils";
import { MoreHorizontal } from "lucide-react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";

export interface ThreadCardMenuAction {
	key: string;
	label: string;
	icon: ReactNode;
	onSelect: () => void;
	disabled?: boolean;
	tone?: "default" | "danger";
}

export function ThreadCardActionMenu({
	actions,
	onOpenChange,
}: {
	actions: ThreadCardMenuAction[];
	onOpenChange?: (isOpen: boolean) => void;
}) {
	const [isOpen, setIsOpen] = useState(false);
	const rootRef = useRef<HTMLDivElement | null>(null);
	const menuId = useId();

	useEffect(() => {
		onOpenChange?.(isOpen);
	}, [isOpen, onOpenChange]);

	useEffect(() => {
		if (!isOpen) {
			return;
		}

		function handlePointerDown(event: PointerEvent) {
			if (
				rootRef.current &&
				event.target instanceof Node &&
				!rootRef.current.contains(event.target)
			) {
				setIsOpen(false);
			}
		}

		function handleEscape(event: KeyboardEvent) {
			if (event.key === "Escape") {
				setIsOpen(false);
			}
		}

		window.addEventListener("pointerdown", handlePointerDown);
		window.addEventListener("keydown", handleEscape);
		return () => {
			window.removeEventListener("pointerdown", handlePointerDown);
			window.removeEventListener("keydown", handleEscape);
		};
	}, [isOpen]);

	if (!actions.length) {
		return null;
	}

	return (
		<div ref={rootRef} className="relative hidden sm:block">
			<button
				type="button"
				className="button-ghost inline-flex size-9 items-center justify-center rounded-full bg-(--surface-ghost-strong-bg) text-(--color-text-secondary) hover:text-(--color-text-primary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40"
				aria-expanded={isOpen}
				aria-controls={menuId}
				aria-label="Thread actions"
				onClick={() => setIsOpen((current) => !current)}
			>
				<MoreHorizontal className="size-4" />
			</button>

			{isOpen ? (
				<div
					id={menuId}
					className="surface-panel surface-panel-soft absolute top-full right-0 z-20 mt-2 min-w-44 rounded-[1rem] p-1.5 shadow-[var(--shadow-float)]"
				>
					{actions.map((action) => (
						<button
							key={action.key}
							type="button"
							disabled={action.disabled}
							className={cn(
								"flex w-full items-center gap-2.5 rounded-[0.8rem] px-3 py-2 text-left text-[0.82rem] font-medium transition-[background-color,color] disabled:opacity-45",
								action.tone === "danger"
									? "text-(--color-destructive) hover:bg-(--color-destructive)/10"
									: "text-(--color-text-secondary) hover:bg-(--surface-ghost-hover-bg) hover:text-(--color-text-primary)",
							)}
							onClick={() => {
								setIsOpen(false);
								action.onSelect();
							}}
						>
							<span className="shrink-0">{action.icon}</span>
							<span>{action.label}</span>
						</button>
					))}
				</div>
			) : null}
		</div>
	);
}

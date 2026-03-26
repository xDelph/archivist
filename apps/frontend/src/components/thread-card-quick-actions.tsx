import type { ThreadCardMenuAction } from "@/components/thread-card-action-menu";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { MouseEvent } from "react";

interface ThreadCardQuickActionsProps {
	actions: ThreadCardMenuAction[];
	variant?: "mobile" | "desktop";
}

export function ThreadCardQuickActions({
	actions,
	variant = "mobile",
}: ThreadCardQuickActionsProps) {
	if (!actions.length) {
		return null;
	}

	if (variant === "desktop") {
		return (
			<div className="hidden items-center gap-1.5 sm:flex">
				{actions.map((action) => (
					<Button
						key={action.key}
						type="button"
						variant="ghost"
						size="icon"
						disabled={action.disabled}
						className={cn(
							"button-ghost size-9 rounded-full border-(--color-border-subtle) bg-(--surface-ghost-strong-bg) p-0 text-(--color-text-secondary) shadow-none sm:size-9",
							action.tone === "danger"
								? "text-(--color-destructive) hover:bg-[color-mix(in_srgb,var(--color-destructive)_12%,var(--color-bg-base))] hover:text-(--color-destructive)"
								: "hover:text-(--color-text-primary)",
						)}
						aria-label={action.label}
						title={action.label}
						onClick={(event) => {
							consumeCardActionEvent(event);
							action.onSelect();
						}}
					>
						<span className="shrink-0">{action.icon}</span>
					</Button>
				))}
			</div>
		);
	}

	return (
		<div
			className={cn(
				"grid gap-2 sm:hidden",
				actions.length === 1 ? "grid-cols-1" : "grid-cols-2",
			)}
		>
			{actions.map((action) => (
				<Button
					key={action.key}
					type="button"
					variant="secondary"
					size="sm"
					disabled={action.disabled}
					className={cn(
						"h-auto min-h-11 w-full justify-center rounded-full px-3 py-2 text-[0.74rem]",
						action.tone === "danger"
							? "border-[color-mix(in_srgb,var(--color-destructive)_34%,transparent)] bg-[color-mix(in_srgb,var(--color-destructive)_12%,var(--color-bg-base))] text-(--color-destructive) hover:bg-[color-mix(in_srgb,var(--color-destructive)_16%,var(--color-bg-base))] hover:text-(--color-destructive)"
							: "border-(--color-border-default) bg-(--color-bg-surface) text-(--color-text-primary)",
					)}
					onClick={action.onSelect}
				>
					<span className="shrink-0">{action.icon}</span>
					<span>{compactActionLabel(action.label)}</span>
				</Button>
			))}
		</div>
	);
}

function compactActionLabel(label: string) {
	return label === "Star thread"
		? "Star"
		: label === "Unstar thread"
			? "Unstar"
			: label;
}

function consumeCardActionEvent(event: MouseEvent<HTMLButtonElement>) {
	event.preventDefault();
	event.stopPropagation();
}

import "./offline-save-notification.css";
import { useOfflineSaveNotification } from "@/lib/offline-save-notification";
import { cn } from "@/lib/utils";
import { AlertCircle, CheckCircle2, Download } from "lucide-react";

export function OfflineSaveNotification() {
	const notification = useOfflineSaveNotification();
	if (!notification) {
		return null;
	}

	const contentMotionKey = `${notification.threadId}:${notification.status}`;
	const Icon =
		notification.status === "success"
			? CheckCircle2
			: notification.status === "error"
				? AlertCircle
				: Download;

	return (
		<div className="pointer-events-none fixed inset-x-0 top-18 z-40 flex justify-center px-3 sm:px-4">
			<div
				className={cn(
					"offline-save-notification surface-panel relative w-full max-w-xl overflow-hidden rounded-[1.15rem] border px-3.5 py-3 shadow-[0_18px_50px_rgba(0,0,0,0.28)]",
					notification.phase === "closing"
						? "offline-save-notification--closing"
						: "offline-save-notification--open",
					notification.status === "success"
						? "border-(--color-border-accent) bg-[color-mix(in_srgb,var(--color-accent)_12%,var(--color-bg-surface))] offline-save-notification--success"
						: notification.status === "error"
							? "border-[color-mix(in_srgb,var(--color-destructive)_40%,var(--color-border-subtle))] bg-[color-mix(in_srgb,var(--color-destructive)_10%,var(--color-bg-surface))] offline-save-notification--error"
							: "border-(--color-border-subtle) bg-(--color-bg-surface) offline-save-notification--pending",
				)}
			>
				<div
					key={contentMotionKey}
					className="offline-save-notification__content relative z-10 flex items-start gap-3"
				>
					<span
						className={cn(
							"offline-save-notification__icon mt-0.5 inline-flex size-8 shrink-0 items-center justify-center rounded-full border",
							notification.status === "success"
								? "border-(--color-border-accent) text-(--color-accent-soft)"
								: notification.status === "error"
									? "border-[color-mix(in_srgb,var(--color-destructive)_44%,var(--color-border-subtle))] text-(--color-destructive)"
									: "border-(--color-border-subtle) text-(--color-text-secondary)",
						)}
					>
						<Icon className="size-4" />
					</span>
					<div className="min-w-0">
						<p className="text-[0.88rem] font-semibold text-(--color-text-primary)">
							{notification.title}
						</p>
						<p className="mt-1 text-[0.8rem] leading-6 text-(--color-text-secondary)">
							{notification.description}
						</p>
					</div>
				</div>
				<div
					className={cn(
						"offline-save-notification__status-wash",
						notification.status === "success"
							? "offline-save-notification__status-wash--success"
							: notification.status === "error"
								? "offline-save-notification__status-wash--error"
								: "offline-save-notification__status-wash--pending",
					)}
				/>
				<div className="offline-save-notification__progress">
					<div
						className={cn(
							"offline-save-notification__progress-bar",
							notification.status === "pending"
								? "offline-save-notification__progress-bar--pending"
								: notification.status === "success"
									? "offline-save-notification__progress-bar--success"
									: "offline-save-notification__progress-bar--error",
						)}
					/>
				</div>
			</div>
		</div>
	);
}

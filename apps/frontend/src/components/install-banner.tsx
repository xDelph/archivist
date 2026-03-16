import { Button } from "@/components/ui/button";
import { useInstallPrompt } from "@/lib/use-install-prompt";
import { Download, X } from "lucide-react";
import { useState } from "react";

export function InstallBanner() {
	const { canInstall, isInstalled, promptInstall } = useInstallPrompt();
	const [dismissed, setDismissed] = useState(false);

	if (!canInstall || isInstalled || dismissed) {
		return null;
	}

	return (
		<div className="mx-auto mb-5 flex max-w-5xl items-center gap-3 rounded-(--radius-card) border border-(--color-border-signal) bg-[linear-gradient(90deg,color-mix(in_srgb,var(--color-accent)_10%,var(--color-bg-panel)),color-mix(in_srgb,var(--color-signal)_12%,var(--color-bg-panel)))] px-4 py-3.5 sm:px-5">
			<Download className="size-5 shrink-0 text-(--color-signal-strong)" />
			<p className="flex-1 text-[0.95rem] leading-relaxed text-(--color-text-primary)">
				<span className="font-medium">Install Archivist</span>{" "}
				<span className="text-(--color-text-secondary)">
					for one-tap access to catch-up and search from your home screen.
				</span>
			</p>
			<Button variant="default" size="sm" onClick={() => void promptInstall()}>
				Install
			</Button>
			<button
				type="button"
				onClick={() => setDismissed(true)}
				className="flex size-11 items-center justify-center rounded-(--radius-button) p-1 text-(--color-text-muted) transition-colors hover:bg-white/6 hover:text-(--color-text-primary) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40 sm:size-10"
				aria-label="Dismiss install prompt"
			>
				<X className="size-4" />
			</button>
		</div>
	);
}

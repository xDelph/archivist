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
		<div className="mx-auto mb-4 flex max-w-5xl items-center gap-3 rounded-(--radius-card) border border-(--color-accent-soft)/20 bg-(--color-accent-soft)/8 px-4 py-3 sm:px-5">
			<Download className="size-5 shrink-0 text-(--color-accent-soft)" />
			<p className="flex-1 text-sm text-(--color-text-primary)">
				<span className="font-medium">Install Archivist</span>{" "}
				<span className="text-(--color-text-secondary)">
					for quick access from your home screen.
				</span>
			</p>
			<Button variant="default" size="sm" onClick={() => void promptInstall()}>
				Install
			</Button>
			<button
				type="button"
				onClick={() => setDismissed(true)}
				className="rounded-(--radius-button) p-1 text-(--color-text-muted) transition-colors hover:text-(--color-text-primary)"
				aria-label="Dismiss install prompt"
			>
				<X className="size-4" />
			</button>
		</div>
	);
}

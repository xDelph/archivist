import { useCallback, useEffect, useState } from "react";

interface BeforeInstallPromptEvent extends Event {
	prompt(): Promise<{ outcome: "accepted" | "dismissed" }>;
}

let deferredPrompt: BeforeInstallPromptEvent | null = null;

export function useInstallPrompt() {
	const [canInstall, setCanInstall] = useState(false);
	const [isInstalled, setIsInstalled] = useState(false);

	useEffect(() => {
		if (isStandalone()) {
			setIsInstalled(true);
			return;
		}

		function handleBeforeInstall(event: Event) {
			event.preventDefault();
			deferredPrompt = event as BeforeInstallPromptEvent;
			setCanInstall(true);
		}

		function handleInstalled() {
			deferredPrompt = null;
			setCanInstall(false);
			setIsInstalled(true);
		}

		window.addEventListener("beforeinstallprompt", handleBeforeInstall);
		window.addEventListener("appinstalled", handleInstalled);

		return () => {
			window.removeEventListener("beforeinstallprompt", handleBeforeInstall);
			window.removeEventListener("appinstalled", handleInstalled);
		};
	}, []);

	const promptInstall = useCallback(async () => {
		if (!deferredPrompt) {
			return false;
		}

		const result = await deferredPrompt.prompt();
		deferredPrompt = null;
		setCanInstall(false);

		return result.outcome === "accepted";
	}, []);

	return { canInstall, isInstalled, promptInstall };
}

function isStandalone() {
	if (typeof window === "undefined") {
		return false;
	}

	return (
		window.matchMedia("(display-mode: standalone)").matches ||
		("standalone" in navigator &&
			(navigator as { standalone?: boolean }).standalone === true)
	);
}

import { useEffect, useState } from "react";

type LegacyMediaQueryList = MediaQueryList & {
	addListener?: (listener: () => void) => void;
	removeListener?: (listener: () => void) => void;
};

export function usePrefersReducedMotion() {
	const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

	useEffect(() => {
		if (
			typeof window === "undefined" ||
			typeof window.matchMedia !== "function"
		) {
			return;
		}

		const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
		const legacyMediaQuery = mediaQuery as LegacyMediaQueryList;
		const updatePreference = () => setPrefersReducedMotion(mediaQuery.matches);

		updatePreference();

		if ("addEventListener" in mediaQuery) {
			mediaQuery.addEventListener("change", updatePreference);
			return () => mediaQuery.removeEventListener("change", updatePreference);
		}

		legacyMediaQuery.addListener?.(updatePreference);
		return () => legacyMediaQuery.removeListener?.(updatePreference);
	}, []);

	return prefersReducedMotion;
}

import {
	type PropsWithChildren,
	createContext,
	useContext,
	useEffect,
	useState,
} from "react";

const THEME_STORAGE_KEY = "arkivist.theme";

const APPEARANCE_MODES = ["dark", "light"] as const;
const ACCENT_MODES = ["green", "red", "blue"] as const;

type AppearanceMode = (typeof APPEARANCE_MODES)[number];
type AccentMode = (typeof ACCENT_MODES)[number];

interface ThemePreference {
	appearance: AppearanceMode;
	accent: AccentMode;
}

const DEFAULT_THEME: ThemePreference = {
	appearance: "dark",
	accent: "green",
};

const THEME_META_COLORS: Record<AppearanceMode, Record<AccentMode, string>> = {
	dark: { green: "#122019", red: "#201516", blue: "#101828" },
	light: { green: "#f4fbf7", red: "#fdf4f4", blue: "#f4f7fe" },
};

function isAppearanceMode(v: unknown): v is AppearanceMode {
	return (
		typeof v === "string" && APPEARANCE_MODES.includes(v as AppearanceMode)
	);
}

function isAccentMode(v: unknown): v is AccentMode {
	return typeof v === "string" && ACCENT_MODES.includes(v as AccentMode);
}

function parseTheme(value: unknown): ThemePreference {
	const c =
		value && typeof value === "object"
			? (value as Record<string, unknown>)
			: {};
	return {
		appearance: isAppearanceMode(c.appearance)
			? c.appearance
			: DEFAULT_THEME.appearance,
		accent: isAccentMode(c.accent) ? c.accent : DEFAULT_THEME.accent,
	};
}

function readStoredTheme(): ThemePreference {
	try {
		const raw = localStorage.getItem(THEME_STORAGE_KEY);
		return raw ? parseTheme(JSON.parse(raw)) : DEFAULT_THEME;
	} catch {
		return DEFAULT_THEME;
	}
}

function applyTheme(theme: ThemePreference) {
	const root = document.documentElement;
	root.dataset.appearance = theme.appearance;
	root.dataset.accent = theme.accent;
	root.style.colorScheme = theme.appearance;
	document
		.querySelector('meta[name="theme-color"]')
		?.setAttribute(
			"content",
			THEME_META_COLORS[theme.appearance][theme.accent],
		);
}

function persistTheme(theme: ThemePreference) {
	try {
		localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(theme));
	} catch {
		/* ignore */
	}
}

interface ThemeContextValue {
	theme: ThemePreference;
	toggleAppearance: () => void;
	cycleAccent: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: PropsWithChildren) {
	const [theme, setTheme] = useState<ThemePreference>(() => {
		const doc = document.documentElement.dataset;
		if (doc.appearance || doc.accent) {
			return parseTheme({ appearance: doc.appearance, accent: doc.accent });
		}
		return readStoredTheme();
	});

	useEffect(() => {
		applyTheme(theme);
		persistTheme(theme);
	}, [theme]);

	useEffect(() => {
		function onStorage(e: StorageEvent) {
			if (e.key === THEME_STORAGE_KEY) setTheme(readStoredTheme());
		}
		window.addEventListener("storage", onStorage);
		return () => window.removeEventListener("storage", onStorage);
	}, []);

	return (
		<ThemeContext.Provider
			value={{
				theme,
				toggleAppearance: () =>
					setTheme((t) => ({
						...t,
						appearance: t.appearance === "dark" ? "light" : "dark",
					})),
				cycleAccent: () =>
					setTheme((t) => {
						const i = ACCENT_MODES.indexOf(t.accent);
						return {
							...t,
							accent: ACCENT_MODES[(i + 1) % ACCENT_MODES.length],
						};
					}),
			}}
		>
			{children}
		</ThemeContext.Provider>
	);
}

export function useTheme() {
	const ctx = useContext(ThemeContext);
	if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
	return ctx;
}

export { ACCENT_MODES, type AccentMode };

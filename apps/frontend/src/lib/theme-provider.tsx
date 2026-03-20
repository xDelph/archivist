import {
	type PropsWithChildren,
	createContext,
	useContext,
	useEffect,
	useState,
} from "react";

export const THEME_STORAGE_KEY = "arkivist.theme";

export const APPEARANCE_MODES = ["dark", "light"] as const;
export const ACCENT_MODES = ["green", "red", "blue"] as const;

export type AppearanceMode = (typeof APPEARANCE_MODES)[number];
export type AccentMode = (typeof ACCENT_MODES)[number];

export interface ThemePreference {
	appearance: AppearanceMode;
	accent: AccentMode;
}

export const DEFAULT_THEME: ThemePreference = {
	appearance: "dark",
	accent: "green",
};

const THEME_META_COLORS: Record<AppearanceMode, Record<AccentMode, string>> = {
	dark: {
		green: "#122019",
		red: "#201516",
		blue: "#101828",
	},
	light: {
		green: "#f4fbf7",
		red: "#fdf4f4",
		blue: "#f4f7fe",
	},
};

function isAppearanceMode(value: unknown): value is AppearanceMode {
	return (
		typeof value === "string" &&
		APPEARANCE_MODES.includes(value as AppearanceMode)
	);
}

function isAccentMode(value: unknown): value is AccentMode {
	return (
		typeof value === "string" && ACCENT_MODES.includes(value as AccentMode)
	);
}

export function parseThemePreference(value: unknown): ThemePreference {
	const candidate =
		value && typeof value === "object"
			? (value as Partial<Record<keyof ThemePreference, unknown>>)
			: {};

	return {
		appearance: isAppearanceMode(candidate.appearance)
			? candidate.appearance
			: DEFAULT_THEME.appearance,
		accent: isAccentMode(candidate.accent)
			? candidate.accent
			: DEFAULT_THEME.accent,
	};
}

export function getThemeMetaColor(theme: ThemePreference): string {
	return THEME_META_COLORS[theme.appearance][theme.accent];
}

export function readStoredTheme(storage?: Pick<Storage, "getItem"> | null) {
	if (!storage) {
		return DEFAULT_THEME;
	}

	try {
		const rawTheme = storage.getItem(THEME_STORAGE_KEY);
		return rawTheme
			? parseThemePreference(JSON.parse(rawTheme))
			: DEFAULT_THEME;
	} catch {
		return DEFAULT_THEME;
	}
}

export function getThemeFromDocument(documentRef?: Document | null) {
	if (!documentRef) {
		return DEFAULT_THEME;
	}

	return parseThemePreference({
		appearance: documentRef.documentElement.dataset.appearance,
		accent: documentRef.documentElement.dataset.accent,
	});
}

export function persistTheme(
	theme: ThemePreference,
	storage?: Pick<Storage, "setItem"> | null,
) {
	if (!storage) {
		return;
	}

	try {
		storage.setItem(THEME_STORAGE_KEY, JSON.stringify(theme));
	} catch {
		// Ignore storage failures and keep the in-memory preference.
	}
}

export function applyThemeToDocument(
	theme: ThemePreference,
	documentRef?: Document | null,
) {
	if (!documentRef) {
		return;
	}

	const root = documentRef.documentElement;
	root.dataset.appearance = theme.appearance;
	root.dataset.accent = theme.accent;
	root.style.colorScheme = theme.appearance;

	documentRef
		.querySelector('meta[name="theme-color"]')
		?.setAttribute("content", getThemeMetaColor(theme));
}

interface ThemeContextValue {
	theme: ThemePreference;
	setAppearance: (appearance: AppearanceMode) => void;
	setAccent: (accent: AccentMode) => void;
	toggleAppearance: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: PropsWithChildren) {
	const [theme, setTheme] = useState<ThemePreference>(() =>
		typeof document === "undefined"
			? DEFAULT_THEME
			: getThemeFromDocument(document),
	);

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		applyThemeToDocument(theme, document);
		persistTheme(theme, window.localStorage);
	}, [theme]);

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		function handleStorage(event: StorageEvent) {
			if (event.key === THEME_STORAGE_KEY) {
				setTheme(readStoredTheme(window.localStorage));
			}
		}

		window.addEventListener("storage", handleStorage);
		return () => window.removeEventListener("storage", handleStorage);
	}, []);

	return (
		<ThemeContext.Provider
			value={{
				theme,
				setAppearance: (appearance) =>
					setTheme((currentTheme) =>
						currentTheme.appearance === appearance
							? currentTheme
							: { ...currentTheme, appearance },
					),
				setAccent: (accent) =>
					setTheme((currentTheme) =>
						currentTheme.accent === accent
							? currentTheme
							: { ...currentTheme, accent },
					),
				toggleAppearance: () =>
					setTheme((currentTheme) => ({
						...currentTheme,
						appearance: currentTheme.appearance === "dark" ? "light" : "dark",
					})),
			}}
		>
			{children}
		</ThemeContext.Provider>
	);
}

export function useTheme() {
	const theme = useContext(ThemeContext);

	if (!theme) {
		throw new Error("useTheme must be used within a ThemeProvider");
	}

	return theme;
}

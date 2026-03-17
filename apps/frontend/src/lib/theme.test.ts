import {
	DEFAULT_THEME,
	applyThemeToDocument,
	getThemeMetaColor,
	parseThemePreference,
	readStoredTheme,
} from "@/lib/theme";
import { describe, expect, it, vi } from "vitest";

describe("parseThemePreference", () => {
	it("accepts valid appearance and accent values", () => {
		expect(
			parseThemePreference({
				appearance: "light",
				accent: "blue",
			}),
		).toEqual({
			appearance: "light",
			accent: "blue",
		});
	});

	it("falls back to defaults when values are invalid", () => {
		expect(
			parseThemePreference({
				appearance: "system",
				accent: "purple",
			}),
		).toEqual(DEFAULT_THEME);
	});
});

describe("readStoredTheme", () => {
	it("returns defaults when storage is empty or invalid", () => {
		expect(
			readStoredTheme({
				getItem: () => null,
			}),
		).toEqual(DEFAULT_THEME);

		expect(
			readStoredTheme({
				getItem: () => "{invalid json",
			}),
		).toEqual(DEFAULT_THEME);
	});

	it("hydrates a saved theme from storage", () => {
		expect(
			readStoredTheme({
				getItem: () => '{"appearance":"light","accent":"red"}',
			}),
		).toEqual({
			appearance: "light",
			accent: "red",
		});
	});
});

describe("applyThemeToDocument", () => {
	it("writes the document dataset, color scheme, and theme-color meta", () => {
		const setAttribute = vi.fn();
		const documentRef = {
			documentElement: {
				dataset: {},
				style: {
					colorScheme: "",
				},
			},
			querySelector: () => ({
				setAttribute,
			}),
		} as unknown as Document;

		applyThemeToDocument(
			{
				appearance: "light",
				accent: "green",
			},
			documentRef,
		);

		expect(documentRef.documentElement.dataset).toEqual({
			appearance: "light",
			accent: "green",
		});
		expect(documentRef.documentElement.style.colorScheme).toBe("light");
		expect(setAttribute).toHaveBeenCalledWith(
			"content",
			getThemeMetaColor({
				appearance: "light",
				accent: "green",
			}),
		);
	});
});

import { z } from "zod";

export const homeTabValues = [
	"starred",
	"fresh",
	"steady",
	"trending",
] as const;

export const homeTabSchema = z.enum(homeTabValues);

export type HomeTab = (typeof homeTabValues)[number];

export function normalizeHomeTab(tab?: string): HomeTab {
	return homeTabSchema.catch("starred").parse(tab);
}

export function toHomeTabSearch(tab: HomeTab) {
	return tab === "starred" ? undefined : tab;
}
